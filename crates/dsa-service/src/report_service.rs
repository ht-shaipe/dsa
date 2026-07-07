//! 报告服务 - 报告模板渲染与i18n标签

use dsa_core::{DsaError, DsaResult, utils, get_global_config};
use deck_mysql::{DataRow, Helper};
use tube::Value;

/// 报告服务
pub struct ReportService;

impl ReportService {
    /// 创建报告服务实例
    pub fn new() -> Self {
        Self
    }

    /// 请求分发 - 可用方法: render, render_brief, render_wechat, history_compare, labels
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "render" => self.render(params).await,
            "render_brief" => self.render_brief(params).await,
            "render_wechat" => self.render_wechat(params).await,
            "history_compare" => self.history_compare(params).await,
            "labels" => self.labels(params),
            _ => Err(DsaError::ApiRouting(format!(
                "report不支持方法: {}",
                method
            ))),
        }
    }

    /// 完整报告渲染 - 生成包含所有章节的Markdown报告
    async fn render(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        let analysis_id = params
            .get("analysis_id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        if code.is_empty() && analysis_id <= 0 {
            return Err(DsaError::Validation(
                "请提供code或analysis_id".to_string(),
            ));
        }

        let row = self.load_analysis(code.as_str(), analysis_id).await?;
        let conf = get_global_config();
        let language = conf.report.report_language.clone();
        let labels = get_labels(&language);

        let stock_code = str_val(&row, "stockCode");
        let stock_name = str_val(&row, "stockName");
        let market_context = str_val(&row, "marketContext");
        let analysis_summary = str_val(&row, "analysisSummary");
        let decision_type = str_val(&row, "decisionType");
        let confidence_level = str_val(&row, "confidenceLevel");
        let risk_warning = str_val(&row, "riskWarning");
        let report_json = str_val(&row, "reportJson");

        let lbl_overview = str_val(&labels, "marketOverview");
        let lbl_trend = str_val(&labels, "trendAnalysis");
        let lbl_decision = str_val(&labels, "decisionSignal");
        let lbl_risk = str_val(&labels, "riskWarnings");
        let lbl_indicators = str_val(&labels, "keyIndicators");
        let lbl_decision_label = translate_decision(&decision_type, &labels);
        let lbl_confidence_label = translate_confidence(&confidence_level, &labels);
        let lbl_confidence = str_val(&labels, "confidence");

        let markdown = format!(
            "# {} - {} {}\n\n\
             ## {}\n\n{}\n\n\
             ## {}\n\n{}\n\n\
             ## {}\n\n**{}** | {}: {}\n\n\
             ## {}\n\n{}\n\n\
             ## {}\n\n{}\n",
            stock_code,
            stock_name,
            lbl_decision_label,
            lbl_overview,
            if market_context.is_empty() { "-" } else { &market_context },
            lbl_trend,
            if analysis_summary.is_empty() { "-" } else { &analysis_summary },
            lbl_decision,
            lbl_decision_label,
            lbl_confidence,
            lbl_confidence_label,
            lbl_risk,
            if risk_warning.is_empty() { "-" } else { &risk_warning },
            lbl_indicators,
            if report_json.is_empty() { "-" } else { &report_json },
        );

        let html_preview = markdown_to_html_preview(&markdown);
        let report_type = "full".to_string();

        Ok(value!({
            "markdown": markdown,
            "htmlPreview": html_preview,
            "code": stock_code,
            "reportType": report_type,
            "language": language,
        }))
    }

    /// 简要报告 - 仅包含市场概况、决策信号和风险提示
    async fn render_brief(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        let analysis_id = params
            .get("analysis_id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        if code.is_empty() && analysis_id <= 0 {
            return Err(DsaError::Validation(
                "请提供code或analysis_id".to_string(),
            ));
        }

        let row = self.load_analysis(code.as_str(), analysis_id).await?;
        let conf = get_global_config();
        let language = conf.report.report_language.clone();
        let labels = get_labels(&language);

        let stock_code = str_val(&row, "stockCode");
        let stock_name = str_val(&row, "stockName");
        let market_context = str_val(&row, "marketContext");
        let decision_type = str_val(&row, "decisionType");
        let confidence_level = str_val(&row, "confidenceLevel");
        let risk_warning = str_val(&row, "riskWarning");

        let lbl_overview = str_val(&labels, "marketOverview");
        let lbl_decision = str_val(&labels, "decisionSignal");
        let lbl_risk = str_val(&labels, "riskWarnings");
        let lbl_decision_label = translate_decision(&decision_type, &labels);
        let lbl_confidence_label = translate_confidence(&confidence_level, &labels);
        let lbl_confidence = str_val(&labels, "confidence");

        let markdown = format!(
            "# {} - {} {}\n\n\
             ## {}\n\n{}\n\n\
             ## {}\n\n**{}** | {}: {}\n\n\
             ## {}\n\n{}\n",
            stock_code,
            stock_name,
            lbl_decision_label,
            lbl_overview,
            if market_context.is_empty() { "-" } else { &market_context },
            lbl_decision,
            lbl_decision_label,
            lbl_confidence,
            lbl_confidence_label,
            lbl_risk,
            if risk_warning.is_empty() { "-" } else { &risk_warning },
        );

        let html_preview = markdown_to_html_preview(&markdown);
        let report_type = "brief".to_string();

        Ok(value!({
            "markdown": markdown,
            "htmlPreview": html_preview,
            "code": stock_code,
            "reportType": report_type,
            "language": language,
        }))
    }

    /// 微信格式报告 - 更短、无HTML、emoji兼容
    async fn render_wechat(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        let analysis_id = params
            .get("analysis_id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        if code.is_empty() && analysis_id <= 0 {
            return Err(DsaError::Validation(
                "请提供code或analysis_id".to_string(),
            ));
        }

        let row = self.load_analysis(code.as_str(), analysis_id).await?;
        let conf = get_global_config();
        let language = conf.report.report_language.clone();
        let labels = get_labels(&language);

        let stock_code = str_val(&row, "stockCode");
        let stock_name = str_val(&row, "stockName");
        let market_context = str_val(&row, "marketContext");
        let decision_type = str_val(&row, "decisionType");
        let confidence_level = str_val(&row, "confidenceLevel");
        let risk_warning = str_val(&row, "riskWarning");

        let decision_emoji = match decision_type.as_str() {
            "buy" => "\u{1f7e2}",
            "sell" => "\u{1f534}",
            "hold" => "\u{1f7e1}",
            _ => "\u{26aa}",
        };
        let lbl_overview = str_val(&labels, "marketOverview");
        let lbl_decision = str_val(&labels, "decisionSignal");
        let lbl_risk = str_val(&labels, "riskWarnings");
        let lbl_decision_label = translate_decision(&decision_type, &labels);
        let lbl_confidence_label = translate_confidence(&confidence_level, &labels);
        let lbl_confidence = str_val(&labels, "confidence");

        let markdown = format!(
            "{} {} | {} {}\n\n\
             {}\n{}\n\n\
             {} {} | {}: {}\n\n\
             {}\n{}",
            stock_code,
            stock_name,
            decision_emoji,
            lbl_decision_label,
            lbl_overview,
            if market_context.is_empty() { "-" } else { &market_context },
            lbl_decision,
            lbl_decision_label,
            lbl_confidence,
            lbl_confidence_label,
            lbl_risk,
            if risk_warning.is_empty() { "-" } else { &risk_warning },
        );

        Ok(value!({
            "markdown": markdown,
            "code": stock_code,
            "report_type": "wechat",
            "language": language,
        }))
    }

    /// 历史对比 - 比较当前分析与前N次分析
    async fn history_compare(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供code".to_string()));
        }

        let conf = get_global_config();
        let compare_n = params
            .get("compare_n")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as u32;
        let n = if compare_n > 0 {
            compare_n
        } else {
            conf.report.history_compare_n
        };
        let language = conf.report.report_language.clone();

        let connector = utils::get_db_connector()?;

        let sql = "SELECT id, stock_code, stock_name, sentiment_score, decision_type, \
                   confidenceLevel, operationAdvice, analysisSummary, riskWarning, \
                   createTime \
                   FROM analysis_history WHERE stock_code = :code AND status = 1 \
                   ORDER BY create_time DESC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![
                ("code".to_string(), Value::from(code.as_str())),
                ("limit".to_string(), Value::from((n + 1) as i64)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询历史对比数据失败: {}", e)))?;

        let labels = get_labels(&language);

        let comparisons: Vec<Value> = rows
            .iter()
            .map(|r| {
                let rv = r.to_value2();
                let dt = str_val(&rv, "decisionType");
                let cl = str_val(&rv, "confidenceLevel");
                let dt_label = translate_decision(&dt, &labels);
                let cl_label = translate_confidence(&cl, &labels);
                value!({
                    "id": rv.get("id").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    "stockCode": str_val(&rv, "stockCode"),
                    "stockName": str_val(&rv, "stockName"),
                    "decisionType": dt,
                    "decisionLabel": dt_label,
                    "confidenceLevel": cl,
                    "confidenceLabel": cl_label,
                    "sentimentScore": rv.get("sentimentScore").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    "createTime": str_val(&rv, "createTime"),
                })
            })
            .collect();

        let summary = build_compare_summary(&comparisons, &labels);

        Ok(value!({
            "comparisons": comparisons,
            "summary": summary,
        }))
    }

    /// 返回i18n标签集
    fn labels(&self, params: &Value) -> DsaResult<Value> {
        let language = utils::param_string(params, "language");
        let lang = if language.is_empty() {
            let conf = get_global_config();
            conf.report.report_language.clone()
        } else {
            language
        };
        Ok(get_labels(&lang))
    }

    /// 加载分析记录 - 按ID或按股票代码取最新，返回Value
    async fn load_analysis(
        &self,
        code: &str,
        analysis_id: i64,
    ) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;

        let (sql, p) = if analysis_id > 0 {
            (
                "SELECT id, stock_code, stock_name, sentiment_score, decision_type, \
                 confidenceLevel, operationAdvice, analysisSummary, riskWarning, \
                 report_json, marketContext, reportType, queryId, status, \
                 createTime, modifyTime \
                 FROM analysis_history WHERE id = :id"
                    .to_string(),
                vec![("id".to_string(), Value::from(analysis_id))],
            )
        } else {
            (
                "SELECT id, stock_code, stock_name, sentiment_score, decision_type, \
                 confidenceLevel, operationAdvice, analysisSummary, riskWarning, \
                 report_json, marketContext, reportType, queryId, status, \
                 createTime, modifyTime \
                 FROM analysis_history WHERE stock_code = :code AND status = 1 \
                 ORDER BY create_time DESC LIMIT 1"
                    .to_string(),
                vec![("code".to_string(), Value::from(code))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询分析记录失败: {}", e)))?;

        rows.into_iter()
            .next()
            .map(|r| r.to_value2())
            .ok_or_else(|| DsaError::Validation("未找到分析记录".to_string()))
    }
}

/// 从Value中获取字符串值
fn str_val(v: &Value, key: &str) -> String {
    v.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

/// 将decisionType映射为i18n标签
fn translate_decision(decision: &str, labels: &Value) -> String {
    match decision {
        "buy" => str_val(labels, "buy"),
        "sell" => str_val(labels, "sell"),
        "hold" => str_val(labels, "hold"),
        "watch" => str_val(labels, "watch"),
        _ => decision.to_string(),
    }
}

/// 将confidenceLevel映射为i18n标签
fn translate_confidence(level: &str, labels: &Value) -> String {
    match level {
        "high" => str_val(labels, "high"),
        "medium" => str_val(labels, "medium"),
        "low" => str_val(labels, "low"),
        _ => level.to_string(),
    }
}

/// 生成对比摘要
fn build_compare_summary(comparisons: &[Value], labels: &Value) -> String {
    if comparisons.is_empty() {
        return "-".to_string();
    }

    let lbl_date = str_val(labels, "date");
    let lbl_decision = str_val(labels, "decisionSignal");
    let lbl_confidence = str_val(labels, "confidence");

    let mut lines = vec![format!(
        "| {} | {} | {} |",
        lbl_date, lbl_decision, lbl_confidence
    )];
    lines.push("| --- | --- | --- |".to_string());

    for c in comparisons {
        let date = str_val(c, "createTime");
        let decision = str_val(c, "decisionLabel");
        let confidence = str_val(c, "confidenceLabel");
        let date_ref = if date.is_empty() { "-" } else { &date };
        let decision_ref = if decision.is_empty() { "-" } else { &decision };
        let confidence_ref = if confidence.is_empty() { "-" } else { &confidence };
        lines.push(format!("| {} | {} | {} |", date_ref, decision_ref, confidence_ref));
    }

    lines.join("\n")
}

/// 简易Markdown到HTML预览转换（仅标题和段落）
fn markdown_to_html_preview(md: &str) -> String {
    let mut html = String::new();
    for line in md.lines() {
        if line.starts_with("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", &line[2..]));
        } else if line.starts_with("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", &line[3..]));
        } else if line.starts_with("**") && line.contains("**") {
            let replaced = line.replace("**", "<strong>").replacen("<strong>", "</strong>", 1);
            html.push_str(&format!("<p>{}</p>\n", replaced));
        } else if !line.is_empty() {
            html.push_str(&format!("<p>{}</p>\n", line));
        }
    }
    html
}

/// 获取i18n标签集 - 支持中文和英文
fn get_labels(language: &str) -> Value {
    match language {
        "en" => value!({
            "marketOverview": "Market Overview",
            "trendAnalysis": "Trend Analysis",
            "decisionSignal": "Decision Signal",
            "riskWarnings": "Risk Warnings",
            "keyIndicators": "Key Indicators",
            "buy": "Buy",
            "sell": "Sell",
            "hold": "Hold",
            "watch": "Watch",
            "high": "High",
            "medium": "Medium",
            "low": "Low",
            "bullish": "Bullish",
            "bearish": "Bearish",
            "neutral": "Neutral",
            "date": "Date",
            "price": "Price",
            "change": "Change",
            "volume": "Volume",
            "confidence": "Confidence",
            "recommendation": "Recommendation",
        }),
        _ => value!({
            "marketOverview": "市场概况",
            "trendAnalysis": "趋势分析",
            "decisionSignal": "决策信号",
            "riskWarnings": "风险提示",
            "keyIndicators": "关键指标",
            "buy": "买入",
            "sell": "卖出",
            "hold": "持有",
            "watch": "观望",
            "high": "高",
            "medium": "中",
            "low": "低",
            "bullish": "看多",
            "bearish": "看空",
            "neutral": "中性",
            "date": "日期",
            "price": "价格",
            "change": "涨跌",
            "volume": "成交量",
            "confidence": "置信度",
            "recommendation": "建议",
        }),
    }
}
