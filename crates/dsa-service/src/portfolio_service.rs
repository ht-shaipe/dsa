//! 组合管理服务 - 持仓/交易/快照

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use qta_crawler::Real;
use tube::Value;

/// 组合管理服务
pub struct PortfolioService {}

impl PortfolioService {
    /// 创建组合管理服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: accounts, add, remove, summary, positions, trades, snapshot, cash_ledger, cash_add, corporate_actions, corporate_action_add, lots, fx_rates, fx_rate_update, risk_analysis, import_trades
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "accounts" => self.accounts(params).await,
            "add" => self.add_position(params).await,
            "remove" => self.remove_position(params).await,
            "summary" => self.summary(params).await,
            "positions" => self.positions(params).await,
            "trades" => self.trades(params).await,
            "snapshot" => self.snapshot(params).await,
            "cash_ledger" => self.cash_ledger(params).await,
            "cash_add" => self.cash_add(params).await,
            "corporate_actions" => self.corporate_actions(params).await,
            "corporate_action_add" => self.corporate_action_add(params).await,
            "lots" => self.lots(params).await,
            "fx_rates" => self.fx_rates(params).await,
            "fx_rate_update" => self.fx_rate_update(params).await,
            "risk_analysis" => self.risk_analysis(params).await,
            "import_trades" => self.import_trades(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "portfolio不支持方法: {}",
                method
            ))),
        }
    }

    /// 查询账户列表
    async fn accounts(&self, _params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, name, market, broker, baseCurrency, initialCapital, \
             remark, status, creatorId, createTime, modifyTime \
             FROM portfolio_accounts WHERE status >= 1 ORDER BY id";
        let rows = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询账户列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 添加持仓 (买入)
    async fn add_position(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let price = params
            .get("price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if price <= 0.0 {
            return Err(DsaError::Validation("请提供有效价格".to_string()));
        }

        let quantity = params
            .get("quantity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if quantity <= 0 {
            return Err(DsaError::Validation("请提供有效数量".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let name = utils::param_string(params, "name");
        let commission = params
            .get("commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let remark = utils::param_string(params, "remark");

        // 1. 插入交易记录
        let trade_sql = "INSERT INTO portfolio_trades \
             (accountId, stockCode, stockName, direction, price, quantity, \
              tradeDate, commission, tradeCurrency, dedupHash, remark, status, createTime) \
             VALUES (:aid, :code, :name, 'buy', :price, :qty, \
              NOW(), :comm, 'CNY', '', :remark, 1, NOW())";
        Helper::execute(
            trade_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
                ("name".to_string(), Value::from(name.as_str())),
                ("price".to_string(), Value::from(price)),
                ("qty".to_string(), Value::from(quantity)),
                ("comm".to_string(), Value::from(commission)),
                ("remark".to_string(), Value::from(remark.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("插入交易记录失败: {}", e)))?;

        // 2. 更新持仓
        let check_sql = "SELECT id, quantity, avgCost FROM portfolio_positions \
             WHERE accountId = :aid AND stockCode = :code AND status = 1 LIMIT 1";
        let existing = Helper::query_rows(
            check_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询持仓失败: {}", e)))?;

        if existing.is_empty() {
            // 新建持仓
            let insert_sql = "INSERT INTO portfolio_positions \
                 (accountId, stockCode, stockName, quantity, avgCost, currentPrice, \
                  marketValue, unrealizedPnl, unrealizedPnlPct, snapshotDate, status, \
                  createTime, modifyTime) \
                 VALUES (:aid, :code, :name, :qty, :avg_cost, :price, :mv, :pnl, :pnl_pct, \
                  NOW(), 1, NOW(), NOW())";
            let mv = price * quantity as f64;
            Helper::execute(
                insert_sql,
                vec![
                    ("aid".to_string(), Value::from(account_id)),
                    ("code".to_string(), Value::from(code.as_str())),
                    ("name".to_string(), Value::from(name.as_str())),
                    ("qty".to_string(), Value::from(quantity)),
                    ("avg_cost".to_string(), Value::from(price)),
                    ("price".to_string(), Value::from(price)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(0.0)),
                    ("pnl_pct".to_string(), Value::from(0.0)),
                ],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("创建持仓失败: {}", e)))?;
        } else {
            // 增加持仓
            let row = &existing[0];
            let pos_id: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let old_qty: i64 = row.get_value(1).as_f64().unwrap_or(0.0) as i64;
            let old_cost: f64 = row.get_value(2).as_f64().unwrap_or(0.0);

            let new_qty = old_qty + quantity;
            let new_avg = if new_qty > 0 {
                (old_cost * old_qty as f64 + price * quantity as f64) / new_qty as f64
            } else {
                price
            };
            let mv = price * new_qty as f64;
            let pnl = (price - new_avg) * new_qty as f64;
            let pnl_pct = if new_avg > 0.0 {
                (price - new_avg) / new_avg * 100.0
            } else {
                0.0
            };

            let update_sql = "UPDATE portfolio_positions SET \
                 quantity = :qty, avgCost = :avg_cost, currentPrice = :price, \
                 marketValue = :mv, unrealizedPnl = :pnl, unrealizedPnlPct = :pnl_pct, \
                 snapshotDate = NOW(), modifyTime = NOW() \
                 WHERE id = :id";
            Helper::execute(
                update_sql,
                vec![
                    ("qty".to_string(), Value::from(new_qty)),
                    ("avg_cost".to_string(), Value::from(new_avg)),
                    ("price".to_string(), Value::from(price)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(pnl)),
                    ("pnl_pct".to_string(), Value::from(pnl_pct)),
                    ("id".to_string(), Value::from(pos_id)),
                ],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("更新持仓失败: {}", e)))?;
        }

        Ok(value!({"status": "ok", "data": {"action": "buy", "code": code, "price": price, "quantity": quantity}}))
    }

    /// 移除持仓 (卖出)
    async fn remove_position(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let price = params
            .get("price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let quantity = params
            .get("quantity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let commission = params
            .get("commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let remark = utils::param_string(params, "remark");

        let connector = utils::get_db_connector()?;

        // 获取持仓信息
        let check_sql = "SELECT id, quantity, avgCost, stockName FROM portfolio_positions \
             WHERE accountId = :aid AND stockCode = :code AND status = 1 LIMIT 1";
        let existing = Helper::query_rows(
            check_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询持仓失败: {}", e)))?;

        if existing.is_empty() {
            return Err(DsaError::Validation(format!(
                "无持仓: {}",
                code
            )));
        }

        let row = &existing[0];
        let pos_id: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
        let old_qty: i64 = row.get_value(1).as_f64().unwrap_or(0.0) as i64;
        let avg_cost: f64 = row.get_value(2).as_f64().unwrap_or(0.0);
        let stock_name = row.get_string(3);

        let sell_qty = if quantity <= 0 || quantity >= old_qty {
            old_qty // 清仓
        } else {
            quantity
        };

        let sell_price = if price <= 0.0 { avg_cost } else { price };

        // 1. 插入交易记录
        let trade_sql = "INSERT INTO portfolio_trades \
             (accountId, stockCode, stockName, direction, price, quantity, \
              tradeDate, commission, tradeCurrency, dedupHash, remark, status, createTime) \
             VALUES (:aid, :code, :name, 'sell', :price, :qty, \
              NOW(), :comm, 'CNY', '', :remark, 1, NOW())";
        Helper::execute(
            trade_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
                ("name".to_string(), Value::from(stock_name.as_str())),
                ("price".to_string(), Value::from(sell_price)),
                ("qty".to_string(), Value::from(sell_qty)),
                ("comm".to_string(), Value::from(commission)),
                ("remark".to_string(), Value::from(remark.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("插入交易记录失败: {}", e)))?;

        // 2. 更新持仓
        let remaining = old_qty - sell_qty;
        if remaining <= 0 {
            // 清仓: 软删除
            let update_sql = "UPDATE portfolio_positions SET quantity = 0, status = 0, modifyTime = NOW() WHERE id = :id";
            Helper::execute(
                update_sql,
                vec![("id".to_string(), Value::from(pos_id))],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("清仓失败: {}", e)))?;
        } else {
            let mv = sell_price * remaining as f64;
            let pnl = (sell_price - avg_cost) * remaining as f64;
            let pnl_pct = if avg_cost > 0.0 {
                (sell_price - avg_cost) / avg_cost * 100.0
            } else {
                0.0
            };

            let update_sql = "UPDATE portfolio_positions SET \
                 quantity = :qty, currentPrice = :price, marketValue = :mv, \
                 unrealizedPnl = :pnl, unrealizedPnlPct = :pnl_pct, \
                 snapshotDate = NOW(), modifyTime = NOW() WHERE id = :id";
            Helper::execute(
                update_sql,
                vec![
                    ("qty".to_string(), Value::from(remaining)),
                    ("price".to_string(), Value::from(sell_price)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(pnl)),
                    ("pnl_pct".to_string(), Value::from(pnl_pct)),
                    ("id".to_string(), Value::from(pos_id)),
                ],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("更新持仓失败: {}", e)))?;
        }

        Ok(value!({"status": "ok", "data": {"action": "sell", "code": code, "price": sell_price, "quantity": sell_qty}}))
    }

    /// 组合摘要 (含实时市值)
    async fn summary(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        let connector = utils::get_db_connector()?;

        let (sql, p) = if account_id > 0 {
            (
                "SELECT id, accountId, stockCode, stockName, quantity, avgCost, \
                 currentPrice, marketValue, unrealizedPnl, unrealizedPnlPct \
                 FROM portfolio_positions WHERE accountId = :aid AND status = 1"
                    .to_string(),
                vec![("aid".to_string(), Value::from(account_id))],
            )
        } else {
            (
                "SELECT id, accountId, stockCode, stockName, quantity, avgCost, \
                 currentPrice, marketValue, unrealizedPnl, unrealizedPnlPct \
                 FROM portfolio_positions WHERE status = 1"
                    .to_string(),
                vec![],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询持仓失败: {}", e)))?;

        let mut total_value = 0.0_f64;
        let mut total_cost = 0.0_f64;
        let mut total_pnl = 0.0_f64;
        let mut positions_detail = Vec::new();

        let real = Real::new();

        for row in &rows {
            let code = row.get_string(2);       // stockCode
            let qty: i64 = row.get_value(4).as_f64().unwrap_or(0.0) as i64;    // quantity
            let avg_cost: f64 = row.get_value(5).as_f64().unwrap_or(0.0);      // avgCost
            let stock_name = row.get_string(3); // stockName

            // 获取实时价格
            let prefix = utils::market_prefix(&code);

            let current_price = match real
                .get_price(&format!("{}{}", prefix, code))
                .await
            {
                Ok(v) => v.get("price").and_then(|p| p.as_f64()).unwrap_or(0.0),
                Err(_) => row.get_value(6).as_f64().unwrap_or(0.0),
            };

            let mv = current_price * qty as f64;
            let cost = avg_cost * qty as f64;
            let pnl = mv - cost;

            total_value += mv;
            total_cost += cost;
            total_pnl += pnl;

            let pnl_pct = if cost > 0.0 { pnl / cost * 100.0 } else { 0.0 };

            positions_detail.push(value!({
                "code": code,
                "name": stock_name,
                "quantity": qty,
                "avgCost": avg_cost,
                "currentPrice": current_price,
                "marketValue": mv,
                "unrealizedPnl": pnl,
                "unrealizedPnlPct": pnl_pct,
            }));
        }

        let total_pnl_pct = if total_cost > 0.0 {
            total_pnl / total_cost * 100.0
        } else {
            0.0
        };

        Ok(value!({
            "status": "ok",
            "data": {
                "totalValue": total_value,
                "totalCost": total_cost,
                "totalPnl": total_pnl,
                "totalPnlPct": total_pnl_pct,
                "positionCount": positions_detail.len() as i64,
                "positions": positions_detail,
            }
        }))
    }

    /// 查询持仓列表
    async fn positions(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        let (sql, p) = if account_id > 0 {
            (
                "SELECT id, accountId, stockCode, stockName, quantity, avgCost, \
                 currentPrice, marketValue, unrealizedPnl, unrealizedPnlPct, \
                 snapshotDate, status, createTime, modifyTime \
                 FROM portfolio_positions WHERE accountId = :aid AND status = 1"
                    .to_string(),
                vec![("aid".to_string(), Value::from(account_id))],
            )
        } else {
            (
                "SELECT id, accountId, stockCode, stockName, quantity, avgCost, \
                 currentPrice, marketValue, unrealizedPnl, unrealizedPnlPct, \
                 snapshotDate, status, createTime, modifyTime \
                 FROM portfolio_positions WHERE status = 1"
                    .to_string(),
                vec![],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询持仓失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 查询交易记录
    async fn trades(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let (sql, p) = if account_id > 0 {
            (
                "SELECT id, accountId, stockCode, stockName, direction, price, quantity, \
                 tradeDate, commission, tradeCurrency, dedupHash, remark, status, createTime \
                 FROM portfolio_trades WHERE accountId = :aid AND status = 1 \
                 ORDER BY createTime DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("aid".to_string(), Value::from(account_id)),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        } else {
            (
                "SELECT id, accountId, stockCode, stockName, direction, price, quantity, \
                 tradeDate, commission, tradeCurrency, dedupHash, remark, status, createTime \
                 FROM portfolio_trades WHERE status = 1 \
                 ORDER BY createTime DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询交易记录失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 每日快照
    async fn snapshot(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let connector = utils::get_db_connector()?;

        // 获取账户初始资金
        let acct_sql = "SELECT initialCapital FROM portfolio_accounts WHERE id = :id AND status >= 1";
        let acct_rows = Helper::query_rows(
            acct_sql,
            vec![("id".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询账户失败: {}", e)))?;

        let initial_capital: f64 = acct_rows
            .first()
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        // 获取持仓汇总
        let pos_sql = "SELECT SUM(marketValue) as mv, SUM(unrealizedPnl) as pnl, \
             SUM(costBasis) as cost FROM portfolio_positions WHERE accountId = :aid AND status = 1";
        let pos_rows = Helper::query_rows(
            pos_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询持仓汇总失败: {}", e)))?;

        let market_value: f64 = pos_rows
            .first()
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);
        let unrealized_pnl: f64 = pos_rows
            .first()
            .map(|r| r.get_value(1).as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);
        let total_cost: f64 = pos_rows
            .first()
            .map(|r| r.get_value(2).as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        // 获取已实现盈亏 (从交易记录汇总)
        let trade_sql = "SELECT SUM(CASE WHEN direction = 'sell' THEN price * quantity - commission \
             ELSE -(price * quantity + commission) END) as realized_pnl \
             FROM portfolio_trades WHERE accountId = :aid AND status = 1";
        let trade_rows = Helper::query_rows(
            trade_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询交易汇总失败: {}", e)))?;

        let realized_pnl: f64 = trade_rows
            .first()
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        // 现金余额 = 初始资金 - 已投入成本 + 卖出收入 - 佣金
        let cash_balance = if total_cost > 0.0 {
            initial_capital - total_cost + realized_pnl
        } else {
            initial_capital
        };
        let total_equity = market_value + cash_balance;
        let total_pnl = total_equity - initial_capital;
        let total_pnl_pct = if initial_capital > 0.0 {
            total_pnl / initial_capital * 100.0
        } else {
            0.0
        };

        // 获取昨日快照用于计算daily_pnl
        let prev_sql = "SELECT totalEquity FROM portfolio_daily_snapshots \
             WHERE accountId = :aid ORDER BY snapshotDate DESC LIMIT 1";
        let prev_rows = Helper::query_rows(
            prev_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .unwrap_or_default();

        let prev_equity: f64 = prev_rows
            .first()
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
            .unwrap_or(initial_capital);

        let daily_pnl = total_equity - prev_equity;
        let daily_pnl_pct = if prev_equity > 0.0 {
            daily_pnl / prev_equity * 100.0
        } else {
            0.0
        };

        // 插入快照
        let insert_sql = "INSERT INTO portfolio_daily_snapshots \
             (accountId, snapshotDate, totalEquity, cashBalance, marketValue, \
              dailyPnl, dailyPnlPct, totalPnl, totalPnlPct, createTime) \
             VALUES (:aid, NOW(), :equity, :cash, :mv, :daily_pnl, :daily_pnl_pct, \
              :total_pnl, :total_pnl_pct, NOW())";
        Helper::execute(
            insert_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("equity".to_string(), Value::from(total_equity)),
                ("cash".to_string(), Value::from(cash_balance)),
                ("mv".to_string(), Value::from(market_value)),
                ("daily_pnl".to_string(), Value::from(daily_pnl)),
                ("daily_pnl_pct".to_string(), Value::from(daily_pnl_pct)),
                ("total_pnl".to_string(), Value::from(total_pnl)),
                ("total_pnl_pct".to_string(), Value::from(total_pnl_pct)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("创建快照失败: {}", e)))?;

        Ok(value!({
            "status": "ok",
            "data": {
                "accountId": account_id,
                "totalEquity": total_equity,
                "cashBalance": cash_balance,
                "marketValue": market_value,
                "unrealizedPnl": unrealized_pnl,
                "dailyPnl": daily_pnl,
                "dailyPnlPct": daily_pnl_pct,
                "totalPnl": total_pnl,
                "totalPnlPct": total_pnl_pct,
            }
        }))
    }

    /// 查询现金流水
    async fn cash_ledger(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, accountId, eventDate, direction, amount, baseCurrency, note, createTime \
             FROM portfolio_cash_ledger WHERE accountId = :aid ORDER BY eventDate DESC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询现金流水失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 新增现金收支事件
    async fn cash_add(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let direction = utils::param_string(params, "direction");
        if direction != "in" && direction != "out" {
            return Err(DsaError::Validation("direction 必须为 in 或 out".to_string()));
        }

        let amount = params
            .get("amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if amount <= 0.0 {
            return Err(DsaError::Validation("请提供有效金额".to_string()));
        }

        let note = utils::param_string(params, "note");
        let connector = utils::get_db_connector()?;

        let sql = "INSERT INTO portfolio_cash_ledger \
             (accountId, eventDate, direction, amount, baseCurrency, note, createTime) \
             VALUES (:aid, NOW(), :dir, :amt, 'CNY', :note, NOW())";
        Helper::execute(
            sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("dir".to_string(), Value::from(direction.as_str())),
                ("amt".to_string(), Value::from(amount)),
                ("note".to_string(), Value::from(note.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("插入现金流水失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"accountId": account_id, "direction": direction, "amount": amount}}))
    }

    /// 查询公司行动记录
    async fn corporate_actions(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let connector = utils::get_db_connector()?;
        let sql = "SELECT * FROM portfolio_corporate_actions \
             WHERE accountId = :aid ORDER BY effectiveDate DESC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询公司行动失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 新增公司行动
    async fn corporate_action_add(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let symbol = utils::param_string(params, "symbol");
        if symbol.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let action_type = utils::param_string(params, "actionType");
        if action_type.is_empty() {
            return Err(DsaError::Validation("请提供行动类型".to_string()));
        }

        let cash_dividend_per_share = params
            .get("cashDividendPerShare")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let split_ratio = utils::param_string(params, "splitRatio");
        let note = utils::param_string(params, "note");
        let connector = utils::get_db_connector()?;

        let sql = "INSERT INTO portfolio_corporate_actions \
             (accountId, symbol, market, baseCurrency, effectiveDate, actionType, \
              cashDividendPerShare, splitRatio, note, createTime) \
             VALUES (:aid, :symbol, 'cn', 'CNY', NOW(), :atype, :cdps, :sr, :note, NOW())";
        Helper::execute(
            sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("symbol".to_string(), Value::from(symbol.as_str())),
                ("atype".to_string(), Value::from(action_type.as_str())),
                ("cdps".to_string(), Value::from(cash_dividend_per_share)),
                ("sr".to_string(), Value::from(split_ratio.as_str())),
                ("note".to_string(), Value::from(note.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("插入公司行动失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"accountId": account_id, "symbol": symbol, "actionType": action_type}}))
    }

    /// 查询FIFO批次
    async fn lots(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT * FROM portfolio_position_lots \
             WHERE accountId = :aid AND remainingQuantity > 0 ORDER BY openDate ASC";
        let rows = Helper::query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询FIFO批次失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 查询汇率
    async fn fx_rates(&self, params: &Value) -> DsaResult<Value> {
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let connector = utils::get_db_connector()?;
        let sql = "SELECT * FROM portfolio_fx_rates ORDER BY rateDate DESC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![("limit".to_string(), Value::from(limit))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询汇率失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 更新汇率
    async fn fx_rate_update(&self, params: &Value) -> DsaResult<Value> {
        let from_currency = utils::param_string(params, "fromCurrency");
        if from_currency.is_empty() {
            return Err(DsaError::Validation("请提供源货币".to_string()));
        }

        let to_currency = utils::param_string(params, "toCurrency");
        if to_currency.is_empty() {
            return Err(DsaError::Validation("请提供目标货币".to_string()));
        }

        let rate = params
            .get("rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if rate <= 0.0 {
            return Err(DsaError::Validation("请提供有效汇率".to_string()));
        }

        let source = utils::param_string(params, "source");
        let connector = utils::get_db_connector()?;

        let sql = "INSERT INTO portfolio_fx_rates \
             (fromCurrency, toCurrency, rateDate, rate, source, isStale, updatedTime) \
             VALUES (:from, :to, NOW(), :rate, :src, 0, NOW()) \
             ON DUPLICATE KEY UPDATE rate = VALUES(rate), source = VALUES(source), \
             isStale = 0, updatedTime = NOW()";
        Helper::execute(
            sql,
            vec![
                ("from".to_string(), Value::from(from_currency.as_str())),
                ("to".to_string(), Value::from(to_currency.as_str())),
                ("rate".to_string(), Value::from(rate)),
                ("src".to_string(), Value::from(source.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("更新汇率失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"fromCurrency": from_currency, "toCurrency": to_currency, "rate": rate}}))
    }

    /// 组合风险分析
    async fn risk_analysis(&self, params: &Value) -> DsaResult<Value> {
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(DsaError::Validation("请提供账户ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT stockCode, marketValue, unrealizedPnlPct \
             FROM portfolio_positions WHERE accountId = :aid AND status = 1";
        let rows = Helper::query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询持仓失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "status": "ok",
                "data": {
                    "position_concentration": 0.0,
                    "sector_breakdown": [],
                    "max_single_loss": 0.0,
                    "position_count": 0,
                }
            }));
        }

        let mut total_mv = 0.0_f64;
        let mut max_mv = 0.0_f64;
        let mut max_loss_pct = 0.0_f64;
        let mut sector_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for row in &rows {
            let code = row.get_string(0);
            let mv: f64 = row.get_value(1).as_f64().unwrap_or(0.0);
            let pnl_pct: f64 = row.get_value(2).as_f64().unwrap_or(0.0);

            total_mv += mv;
            if mv > max_mv {
                max_mv = mv;
            }
            if pnl_pct < max_loss_pct {
                max_loss_pct = pnl_pct;
            }

            // 按股票代码范围映射行业 (A股简映射)
            let sector = if code.starts_with('6') || code.starts_with("60") {
                // 沪市主板 - 金融为主 (简化)
                "金融"
            } else if code.starts_with("00") {
                // 深市主板 - 消费为主 (简化)
                "消费"
            } else if code.starts_with("30") {
                // 创业板 - 科技为主 (简化)
                "科技"
            } else if code.starts_with("68") {
                // 科创板 - 医药/科技 (简化)
                "医药"
            } else {
                "其他"
            };

            *sector_map.entry(sector.to_string()).or_insert(0.0) += mv;
        }

        let position_concentration = if total_mv > 0.0 {
            max_mv / total_mv * 100.0
        } else {
            0.0
        };

        let mut sector_breakdown: Vec<Value> = Vec::new();
        for (sector, sector_mv) in &sector_map {
            let pct = if total_mv > 0.0 {
                sector_mv / total_mv * 100.0
            } else {
                0.0
            };
            sector_breakdown.push(value!({
                "sector": sector,
                "marketValue": *sector_mv,
                "percentage": pct,
            }));
        }

        Ok(value!({
            "status": "ok",
            "data": {
                "position_concentration": position_concentration,
                "sector_breakdown": sector_breakdown,
                "max_single_loss": max_loss_pct,
                "position_count": rows.len() as i64,
                "totalMarketValue": total_mv,
            }
        }))
    }

    /// 批量导入交易
    async fn import_trades(&self, params: &Value) -> DsaResult<Value> {
        let trades = params
            .get("trades")
            .ok_or_else(|| DsaError::Validation("请提供trades数组".to_string()))?;

        let trades_arr = trades
            .as_array()
            .ok_or_else(|| DsaError::Validation("trades必须为数组".to_string()))?;

        if trades_arr.is_empty() {
            return Err(DsaError::Validation("trades数组不能为空".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let mut success_count = 0_i64;
        let mut fail_count = 0_i64;
        let mut errors: Vec<Value> = Vec::new();

        for (idx, trade) in trades_arr.iter().enumerate() {
            let account_id = trade
                .get("accountId")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as i64;
            if account_id == 0 {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "缺少accountId"}));
                continue;
            }

            let code = utils::param_string(trade, "code");
            if code.is_empty() {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "缺少code"}));
                continue;
            }

            let direction = utils::param_string(trade, "direction");
            if direction.is_empty() {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "缺少direction"}));
                continue;
            }

            let price = trade
                .get("price")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if price <= 0.0 {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "缺少有效price"}));
                continue;
            }

            let quantity = trade
                .get("quantity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as i64;
            if quantity <= 0 {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "缺少有效quantity"}));
                continue;
            }

            let name = utils::param_string(trade, "name");
            let commission = trade
                .get("commission")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let remark = utils::param_string(trade, "remark");

            let sql = "INSERT INTO portfolio_trades \
                 (accountId, stockCode, stockName, direction, price, quantity, \
                  tradeDate, commission, tradeCurrency, dedupHash, remark, status, createTime) \
                 VALUES (:aid, :code, :name, :dir, :price, :qty, \
                  NOW(), :comm, 'CNY', '', :remark, 1, NOW())";

            match Helper::execute(
                sql,
                vec![
                    ("aid".to_string(), Value::from(account_id)),
                    ("code".to_string(), Value::from(code.as_str())),
                    ("name".to_string(), Value::from(name.as_str())),
                    ("dir".to_string(), Value::from(direction.as_str())),
                    ("price".to_string(), Value::from(price)),
                    ("qty".to_string(), Value::from(quantity)),
                    ("comm".to_string(), Value::from(commission)),
                    ("remark".to_string(), Value::from(remark.as_str())),
                ],
                &connector,
            ) {
                Ok(_) => success_count += 1,
                Err(e) => {
                    fail_count += 1;
                    errors.push(value!({"index": idx as i64, "error": format!("插入失败: {}", e)}));
                }
            }
        }

        Ok(value!({
            "status": "ok",
            "data": {
                "total": trades_arr.len() as i64,
                "success": success_count,
                "failed": fail_count,
                "errors": errors,
            }
        }))
    }
}
