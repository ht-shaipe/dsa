use dsa_core::utils;
use deck_mysql::{DataRow, Helper};
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct DecisionExtractor {
    request: RequestParameter,
}

impl DecisionExtractor {
    pub fn new(param: &RequestParameter) -> Self {
        DecisionExtractor { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "extract" => self.extract().await,
            "extract_batch" => self.extract_batch().await,
            "data_quality" => self.data_quality().await,
            _ => Err(tube::Error::from(format!("decision_extractor不支持方法: {}", method))),
        }
    }

    fn params(&self) -> &Value { &self.request.value }

    async fn extract(&self) -> Result<Value> {
        let params = self.params();
        let analysis_id = utils::param_i64(params, "analysisId");
        if analysis_id == 0 {
            return Err(tube::Error::from("请提供analysisId"));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let sql = "SELECT id, stock_code, stock_name, decision_type, confidence_level,             sentiment_score, ideal_buy, stop_loss, take_profit, operation_advice,             risk_warning, market_context, analysis_summary, create_time             FROM analysis_history WHERE id = :id AND status >= 1";
        let rows = Helper::query_rows(
            sql,
            vec![("id".to_string(), Value::from(analysis_id))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询分析历史失败: {}", e)))?;

        if rows.is_empty() {
            return Err(tube::Error::from(format!("分析记录不存在: {}", analysis_id)));
        }

        let row = &rows[0];
        let stock_code = row.get_string(1);
        let stock_name = row.get_string(2);
        let decision_type = row.get_string(3);
        let confidence_level = row.get_string(4);
        let sentiment_score: i32 = row.get_value(5).as_f64().unwrap_or(0.0) as i32;
        let ideal_buy: f64 = row.get_value(6).as_f64().unwrap_or(0.0);
        let stop_loss: f64 = row.get_value(7).as_f64().unwrap_or(0.0);
        let take_profit: f64 = row.get_value(8).as_f64().unwrap_or(0.0);
        let operation_advice = row.get_string(9);
        let risk_warning = row.get_string(10);
        let market_context = row.get_string(11);
        let analysis_summary = row.get_string(12);
        let create_time = row.get_string(13);

        let action = match decision_type.as_str() {
            "强烈推荐" | "推荐" | "买入" => "buy",
            "增持" | "加仓" => "add",
            "持有" | "观望" => "hold",
            "减持" | "减仓" => "reduce",
            "卖出" | "回避" => "sell",
            "关注" => "watch",
            "警示" => "avoid",
            _ => "hold",
        };

        let market_phase = if market_context.contains("上涨") || market_context.contains("牛市") {
            "bull"
        } else if market_context.contains("下跌") || market_context.contains("熊市") {
            "bear"
        } else {
            "sideways"
        };

        let dedup_sql = "SELECT id FROM decision_signals             WHERE analysis_id = :aid AND source_type = 'analysis' AND status >= 1 LIMIT 1";
        let dedup_rows = Helper::query_rows(
            dedup_sql,
            vec![("aid".to_string(), Value::from(analysis_id))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("去重检查失败: {}", e)))?;

        if !dedup_rows.is_empty() {
            let existing_id: i64 = dedup_rows[0].get_value(0).as_f64().unwrap_or(0.0) as i64;
            return Ok(value!({
                "status": "ok",
                "data": {"id": existing_id, "message": "信号已存在", "duplicate": true}
            }));
        }

        let insert_sql = "INSERT INTO decision_signals             (stock_code, stock_name, signal_date, action, sentiment_score, confidence_level,              entry_price, stop_loss, target_price, reasoning, evidence, scope_type,              analysis_id, status, market, source_type, market_phase, confidence,              risk_summary, signal_status, create_time, modify_time)             VALUES (:code, :name, :sdate, :action, :score, :conf,              :entry, :sl, :tp, :reasoning, :evidence, 'watchlist',              :aid, 1, 'A', 'analysis', :phase, :confidence,              :risk, 'active', NOW(), NOW())";

        let result = Helper::execute(
            insert_sql,
            vec![
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("name".to_string(), Value::from(stock_name.as_str())),
                ("sdate".to_string(), Value::from(create_time.as_str())),
                ("action".to_string(), Value::from(action)),
                ("score".to_string(), Value::from(sentiment_score)),
                ("conf".to_string(), Value::from(confidence_level.as_str())),
                ("entry".to_string(), Value::from(ideal_buy)),
                ("sl".to_string(), Value::from(stop_loss)),
                ("tp".to_string(), Value::from(take_profit)),
                ("reasoning".to_string(), Value::from(operation_advice.as_str())),
                ("evidence".to_string(), Value::from(analysis_summary.as_str())),
                ("aid".to_string(), Value::from(analysis_id)),
                ("phase".to_string(), Value::from(market_phase)),
                ("confidence".to_string(), Value::from(match confidence_level.as_str() {
                    "high" | "极高" | "高" => 0.9,
                    "medium" | "中" => 0.6,
                    "low" | "低" => 0.3,
                    _ => 0.5,
                })),
                ("risk".to_string(), Value::from(risk_warning.as_str())),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("创建决策信号失败: {}", e)))?;

        Ok(value!({
            "status": "ok",
            "data": {
                "id": result as i64,
                "stock_code": stock_code,
                "action": action,
                "market_phase": market_phase,
                "duplicate": false,
            }
        }))
    }

    async fn extract_batch(&self) -> Result<Value> {
        let params = self.params();
        let limit = utils::param_i64(params, "limit").max(1).min(100) as i64;
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let sql = "SELECT ah.id             FROM analysis_history ah             LEFT JOIN decision_signals ds ON ah.id = ds.analysis_id AND ds.source_type = 'analysis' AND ds.status >= 1             WHERE ah.status >= 1 AND ds.id IS NULL             ORDER BY ah.create_time DESC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![("limit".to_string(), Value::from(limit))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询孤儿分析失败: {}", e)))?;

        let mut extracted_count: i64 = 0;
        let mut errors: Vec<Value> = Vec::new();

        for row in &rows {
            let aid: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let extract_params = value!({"analysisId": aid});
            let temp_req = {
                let mut r = self.request.clone();
                r.method = "extract".to_string();
                r.value = extract_params;
                r
            };
            let temp = DecisionExtractor { request: temp_req };
            match temp.extract().await {
                Ok(_) => {
                    extracted_count += 1;
                }
                Err(e) => {
                    errors.push(value!({"analysisId": aid, "error": e.to_string()}));
                }
            }
        }

        Ok(value!({
            "status": "ok",
            "data": {
                "extractedCount": extracted_count,
                "totalOrphans": rows.len() as i64,
                "errors": errors,
            }
        }))
    }

    async fn data_quality(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::from("请提供股票代码"));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let mut score: i32 = 0;

        let coverage_sql = "SELECT COUNT(*) FROM stock_daily WHERE stock_code = :code AND trade_date >= DATE_SUB(CURDATE(), INTERVAL 30 DAY) AND status = 1";
        let coverage_rows = Helper::query_rows(
            coverage_sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询K线覆盖度失败: {}", e)))?;

        let daily_count: i64 = if !coverage_rows.is_empty() {
            coverage_rows[0].get_value(0).as_f64().unwrap_or(0.0) as i64
        } else {
            0
        };
        let kline_score = if daily_count >= 20 { 2 } else if daily_count >= 10 { 1 } else { 0 };
        score += kline_score;

        let news_sql = "SELECT COUNT(*) FROM news_intel WHERE stock_code = :code AND create_time >= DATE_SUB(CURDATE(), INTERVAL 7 DAY) AND status >= 1";
        let news_rows = Helper::query_rows(
            news_sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询新闻覆盖度失败: {}", e)))?;

        let news_count: i64 = if !news_rows.is_empty() {
            news_rows[0].get_value(0).as_f64().unwrap_or(0.0) as i64
        } else {
            0
        };
        let news_score = if news_count >= 3 { 1 } else { 0 };
        score += news_score;

        let analysis_sql = "SELECT COUNT(*) FROM analysis_history WHERE stock_code = :code AND create_time >= DATE_SUB(CURDATE(), INTERVAL 3 DAY) AND status >= 1";
        let analysis_rows = Helper::query_rows(
            analysis_sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询分析覆盖度失败: {}", e)))?;

        let analysis_count: i64 = if !analysis_rows.is_empty() {
            analysis_rows[0].get_value(0).as_f64().unwrap_or(0.0) as i64
        } else {
            0
        };
        let analysis_score = if analysis_count > 0 { 1 } else { 0 };
        score += analysis_score;

        let signal_sql = "SELECT COUNT(*) FROM decision_signals WHERE stock_code = :code AND status = 1";
        let signal_rows = Helper::query_rows(
            signal_sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询信号覆盖度失败: {}", e)))?;

        let signal_count: i64 = if !signal_rows.is_empty() {
            signal_rows[0].get_value(0).as_f64().unwrap_or(0.0) as i64
        } else {
            0
        };
        let signal_score = if signal_count > 0 { 1 } else { 0 };
        score += signal_score;

        Ok(value!({
            "status": "ok",
            "data": {
                "code": code,
                "totalScore": score,
                "maxScore": 5,
                "breakdown": {
                    "klineCoverage": daily_count,
                    "klineScore": kline_score,
                    "newsCount7d": news_count,
                    "newsScore": news_score,
                    "analysisCount3d": analysis_count,
                    "analysisScore": analysis_score,
                    "activeSignals": signal_count,
                    "signalScore": signal_score,
                },
            }
        }))
    }
}
