use dsa_core::db::{execute, query_rows, row_get_f64, row_get_i64, row_get_string};
use dsa_core::models::db::PortfolioAccount as PortfolioAccountModel;
use dsa_core::models::db::PortfolioCashLedger as PortfolioCashLedgerModel;
use dsa_core::models::db::PortfolioCorporateAction as PortfolioCorporateActionModel;
use dsa_core::models::db::PortfolioDailySnapshot as PortfolioDailySnapshotModel;
use dsa_core::models::db::PortfolioFxRate as PortfolioFxRateModel;
use dsa_core::models::db::PortfolioPosition as PortfolioPositionModel;
use dsa_core::models::db::PortfolioPositionLot as PortfolioPositionLotModel;
use dsa_core::models::db::PortfolioTrade as PortfolioTradeModel;
use dsa_core::utils;
use deck::sqlite::DataTable;
use deck::QueryExecutor;
use deck::TableService;

use qta_crawler::Real;
use tube::{Result, Value};
use tube_web::RequestParameter;

struct TradeTable { request: RequestParameter }

impl DataTable<PortfolioTradeModel> for TradeTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioTradeModel> for TradeTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl TradeTable {
    fn new(param: &RequestParameter) -> Self { TradeTable { request: param.clone() } }

    fn query_trades(&self, account_id: i64, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let (sql, p) = if account_id > 0 {
            ("SELECT id, account_id, stock_code, stock_name, direction, price, quantity, \
              trade_date, commission, trade_currency, dedup_hash, remark, status, create_time \
              FROM portfolio_trades WHERE account_id = :aid AND status = 1 \
              ORDER BY create_time DESC LIMIT :limit".to_string(),
             vec![("aid".to_string(), Value::from(account_id)),
                  ("limit".to_string(), Value::from(limit))])
        } else {
            ("SELECT id, account_id, stock_code, stock_name, direction, price, quantity, \
              trade_date, commission, trade_currency, dedup_hash, remark, status, create_time \
              FROM portfolio_trades WHERE status = 1 \
              ORDER BY create_time DESC LIMIT :limit".to_string(),
             vec![("limit".to_string(), Value::from(limit))])
        };
        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询交易记录失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_trade(&self, account_id: i64, code: &str, name: &str, direction: &str,
                    price: f64, quantity: i64, commission: f64, remark: &str, trade_date: Option<chrono::NaiveDateTime>) -> Result<Value> {
        let trade_date_val = trade_date.unwrap_or_else(|| chrono::Local::now().naive_local());
        let data = value!({
            "account_id": account_id,
            "stock_code": code,
            "stock_name": name,
            "direction": direction,
            "price": price,
            "quantity": quantity,
            "trade_date": trade_date_val,
            "commission": commission,
            "trade_currency": "CNY",
            "dedup_hash": "",
            "remark": remark,
            "status": 1,
            "create_time": chrono::Local::now().naive_local(),
        });
        self.insert().data(&data).execute()
            .map_err(|e| tube::Error::msg(format!("插入交易记录失败: {}", e)))
    }

    fn query_realized_pnl(&self, account_id: i64) -> Result<f64> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT SUM(CASE WHEN direction = 'sell' THEN price * quantity - commission \
             ELSE -(price * quantity + commission) END) as realized_pnl \
             FROM portfolio_trades WHERE account_id = :aid AND status = 1";
        let rows = query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询交易汇总失败: {}", e)))?;
        Ok(rows.first().map(|r| row_get_f64(r, "realizedPnl")).unwrap_or(0.0))
    }

    fn import_one_trade(&self, account_id: i64, code: &str, name: &str, direction: &str,
                        price: f64, quantity: i64, commission: f64, remark: &str) -> Result<Value> {
        self.insert_trade(account_id, code, name, direction, price, quantity, commission, remark, None)
    }
}

struct AccountTable { request: RequestParameter }

impl DataTable<PortfolioAccountModel> for AccountTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioAccountModel> for AccountTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl AccountTable {
    fn new(param: &RequestParameter) -> Self { AccountTable { request: param.clone() } }

    fn query_accounts(&self) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, name, market, broker, base_currency, initial_capital, \
             remark, status, creator_id, create_time, modify_time \
             FROM portfolio_accounts WHERE status >= 1 ORDER BY id";
        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::msg(format!("查询账户列表失败: {}", e)))?;
        Ok(rows)
    }

    fn get_initial_capital(&self, account_id: i64) -> Result<f64> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT initial_capital FROM portfolio_accounts WHERE id = :id AND status >= 1";
        let rows = query_rows(
            sql,
            vec![("id".to_string(), Value::from(account_id))],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询账户失败: {}", e)))?;
        Ok(rows.first().map(|r| row_get_f64(r, "initialCapital")).unwrap_or(0.0))
    }
}

struct CashLedgerTable { request: RequestParameter }

impl DataTable<PortfolioCashLedgerModel> for CashLedgerTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioCashLedgerModel> for CashLedgerTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl CashLedgerTable {
    fn new(param: &RequestParameter) -> Self { CashLedgerTable { request: param.clone() } }

    fn query_cash_ledger(&self, account_id: i64, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, account_id, event_date, direction, amount, base_currency, note, create_time \
             FROM portfolio_cash_ledger WHERE account_id = :aid ORDER BY event_date DESC LIMIT :limit";
        let rows = query_rows(
            sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询现金流水失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_cash_event(&self, account_id: i64, direction: &str, amount: f64, note: &str) -> Result<Value> {
        let data = value!({
            "account_id": account_id,
            "event_date": chrono::Local::now().naive_local(),
            "direction": direction,
            "amount": amount,
            "base_currency": "CNY",
            "note": note,
            "create_time": chrono::Local::now().naive_local(),
        });
        self.insert().data(&data).execute()
            .map_err(|e| tube::Error::msg(format!("插入现金流水失败: {}", e)))
    }
}

struct CorporateActionTable { request: RequestParameter }

impl DataTable<PortfolioCorporateActionModel> for CorporateActionTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioCorporateActionModel> for CorporateActionTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl CorporateActionTable {
    fn new(param: &RequestParameter) -> Self { CorporateActionTable { request: param.clone() } }

    fn query_corporate_actions(&self, account_id: i64, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT * FROM portfolio_corporate_actions \
             WHERE account_id = :aid ORDER BY effective_date DESC LIMIT :limit";
        let rows = query_rows(
            sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询公司行动失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_corporate_action(&self, account_id: i64, symbol: &str, action_type: &str,
                                cash_dividend_per_share: f64, split_ratio: &str, note: &str) -> Result<Value> {
        let data = value!({
            "account_id": account_id,
            "symbol": symbol,
            "market": "cn",
            "base_currency": "CNY",
            "effective_date": chrono::Local::now().naive_local(),
            "action_type": action_type,
            "cash_dividend_per_share": cash_dividend_per_share,
            "split_ratio": split_ratio,
            "note": note,
            "create_time": chrono::Local::now().naive_local(),
        });
        self.insert().data(&data).execute()
            .map_err(|e| tube::Error::msg(format!("插入公司行动失败: {}", e)))
    }
}

struct DailySnapshotTable { request: RequestParameter }

impl DataTable<PortfolioDailySnapshotModel> for DailySnapshotTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioDailySnapshotModel> for DailySnapshotTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl DailySnapshotTable {
    fn new(param: &RequestParameter) -> Self { DailySnapshotTable { request: param.clone() } }

    fn insert_snapshot(&self, account_id: i64, total_equity: f64, cash_balance: f64,
                       market_value: f64, daily_pnl: f64, daily_pnl_pct: f64,
                       total_pnl: f64, total_pnl_pct: f64) -> Result<Value> {
        let data = value!({
            "account_id": account_id,
            "snapshot_date": chrono::Local::now().naive_local(),
            "total_equity": total_equity,
            "cash_balance": cash_balance,
            "market_value": market_value,
            "daily_pnl": daily_pnl,
            "daily_pnl_pct": daily_pnl_pct,
            "total_pnl": total_pnl,
            "total_pnl_pct": total_pnl_pct,
            "create_time": chrono::Local::now().naive_local(),
        });
        self.insert().data(&data).execute()
            .map_err(|e| tube::Error::msg(format!("创建快照失败: {}", e)))
    }

    fn query_prev_equity(&self, account_id: i64, fallback: f64) -> Result<f64> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT total_equity FROM portfolio_daily_snapshots \
             WHERE account_id = :aid ORDER BY snapshot_date DESC LIMIT 1";
        let rows = query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        ).unwrap_or_default();
        Ok(rows.first().map(|r| row_get_f64(r, "totalEquity")).unwrap_or(fallback))
    }
}

struct FxRateTable { request: RequestParameter }

impl DataTable<PortfolioFxRateModel> for FxRateTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioFxRateModel> for FxRateTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl FxRateTable {
    fn new(param: &RequestParameter) -> Self { FxRateTable { request: param.clone() } }

    fn query_fx_rates(&self, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT * FROM portfolio_fx_rates ORDER BY rate_date DESC LIMIT :limit";
        let rows = query_rows(
            sql,
            vec![("limit".to_string(), Value::from(limit))],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询汇率失败: {}", e)))?;
        Ok(rows)
    }

    fn upsert_fx_rate(&self, from_currency: &str, to_currency: &str, rate: f64, source: &str) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "INSERT INTO portfolio_fx_rates \
             (from_currency, to_currency, rate_date, rate, source, is_stale, updated_time) \
             VALUES (:from, :to, NOW(), :rate, :src, 0, NOW()) \
             ON DUPLICATE KEY UPDATE rate = VALUES(rate), source = VALUES(source), \
             is_stale = 0, updated_time = NOW()";
        execute(
            sql,
            vec![
                ("from".to_string(), Value::from(from_currency)),
                ("to".to_string(), Value::from(to_currency)),
                ("rate".to_string(), Value::from(rate)),
                ("src".to_string(), Value::from(source)),
            ],
            &connector,
        ).map(|n| tube::Value::from(n as i64)).map_err(|e| tube::Error::msg(format!("更新汇率失败: {}", e)))
    }
}

struct PositionLotTable { request: RequestParameter }

impl DataTable<PortfolioPositionLotModel> for PositionLotTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<PortfolioPositionLotModel> for PositionLotTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl PositionLotTable {
    fn new(param: &RequestParameter) -> Self { PositionLotTable { request: param.clone() } }

    fn query_lots(&self, account_id: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT * FROM portfolio_position_lots \
             WHERE account_id = :aid AND remaining_quantity > 0 ORDER BY open_date ASC";
        let rows = query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询FIFO批次失败: {}", e)))?;
        Ok(rows)
    }
}

pub struct Portfolio { request: RequestParameter }

impl DataTable<PortfolioPositionModel> for Portfolio {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}

impl TableService<PortfolioPositionModel> for Portfolio {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}

impl Portfolio {
    pub fn new(param: &RequestParameter) -> Self {
        Portfolio { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "accounts" => self.accounts().await,
            "add" => self.add_position().await,
            "remove" => self.remove_position().await,
            "summary" => self.summary().await,
            "positions" => self.positions().await,
            "trades" => self.trades().await,
            "snapshot" => self.snapshot().await,
            "cash_ledger" => self.cash_ledger().await,
            "cash_add" => self.cash_add().await,
            "corporate_actions" => self.corporate_actions().await,
            "corporate_action_add" => self.corporate_action_add().await,
            "lots" => self.lots().await,
            "fx_rates" => self.fx_rates().await,
            "fx_rate_update" => self.fx_rate_update().await,
            "risk_analysis" => self.risk_analysis().await,
            "import_trades" => self.import_trades().await,
            _ => Err(tube::Error::msg(format!("portfolio不支持方法: {}", method))),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn accounts(&self) -> Result<Value> {
        let table = AccountTable::new(&self.request);
        let results = table.query_accounts()?;
        Ok(Value::Array(results))
    }

    async fn add_position(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::msg("请提供股票代码".to_string()));
        }

        let price = params
            .get("price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if price <= 0.0 {
            return Err(tube::Error::msg("请提供有效价格".to_string()));
        }

        let quantity = params
            .get("quantity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if quantity <= 0 {
            return Err(tube::Error::msg("请提供有效数量".to_string()));
        }

        let name = utils::param_string(params, "name");
        let commission = params
            .get("commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let remark = utils::param_string(params, "remark");

        let trade_date = params.get("tradeDate").and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
                .or_else(|| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap())));

        let trade_table = TradeTable::new(&self.request);
        trade_table.insert_trade(account_id, &code, &name, "buy", price, quantity, commission, &remark, trade_date)?;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let check_sql = "SELECT id, quantity, avg_cost FROM portfolio_positions \
             WHERE account_id = :aid AND stock_code = :code AND status = 1 LIMIT 1";
        let existing = query_rows(
            check_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        if existing.is_empty() {
            let mv = price * quantity as f64;
            let cost_with_comm = price * quantity as f64 + commission;
            let avg = cost_with_comm / quantity as f64;
            let data = value!({
                "account_id": account_id,
                "stock_code": code.clone(),
                "stock_name": name,
                "quantity": quantity,
                "avg_cost": avg,
                "current_price": price,
                "market_value": mv,
                "unrealized_pnl": 0.0,
                "unrealized_pnl_pct": 0.0,
                "snapshot_date": chrono::Local::now().naive_local(),
                "status": 1,
                "create_time": chrono::Local::now().naive_local(),
                "modify_time": chrono::Local::now().naive_local(),
            });
            self.insert().data(&data).execute()
                .map_err(|e| tube::Error::msg(format!("创建持仓失败: {}", e)))?;
        } else {
            let row = &existing[0];
            let pos_id: i64 = row_get_i64(row, "id");
            let old_qty: i64 = row_get_f64(row, "quantity") as i64;
            let old_cost: f64 = row_get_f64(row, "avgCost");

            let new_qty = old_qty + quantity;
            let buy_total = price * quantity as f64 + commission;
            let new_avg = if new_qty > 0 {
                (old_cost * old_qty as f64 + buy_total) / new_qty as f64
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
                 quantity = :qty, avg_cost = :avg_cost, current_price = :price, \
                 market_value = :mv, unrealized_pnl = :pnl, unrealized_pnl_pct = :pnl_pct, \
                 snapshot_date = NOW(), modify_time = NOW() \
                 WHERE id = :id";
            execute(
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
            ).map_err(|e| tube::Error::msg(format!("更新持仓失败: {}", e)))?;
        }

        Ok(value!({"action": "buy", "code": code, "price": price, "quantity": quantity}))
    }

    async fn remove_position(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::msg("请提供股票代码".to_string()));
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

        let trade_date = params.get("tradeDate").and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
                .or_else(|| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap())));

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let check_sql = "SELECT id, quantity, avg_cost, stock_name FROM portfolio_positions \
             WHERE account_id = :aid AND stock_code = :code AND status = 1 LIMIT 1";
        let existing = query_rows(
            check_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        if existing.is_empty() {
            return Err(tube::Error::msg(format!("无持仓: {}", code)));
        }

        let row = &existing[0];
        let pos_id: i64 = row_get_i64(row, "id");
        let old_qty: i64 = row_get_f64(row, "quantity") as i64;
        let avg_cost: f64 = row_get_f64(row, "avgCost");
        let stock_name = row_get_string(row, "stockName");

        let sell_qty = if quantity <= 0 || quantity >= old_qty {
            old_qty
        } else {
            quantity
        };

        let sell_price = if price <= 0.0 { avg_cost } else { price };

        let trade_table = TradeTable::new(&self.request);
        trade_table.insert_trade(account_id, &code, &stock_name, "sell", sell_price, sell_qty, commission, &remark, trade_date)?;

        let remaining = old_qty - sell_qty;
        if remaining <= 0 {
            let update_sql = "UPDATE portfolio_positions SET quantity = 0, status = 0, modify_time = NOW() WHERE id = :id";
            execute(
                update_sql,
                vec![("id".to_string(), Value::from(pos_id))],
                &connector,
            ).map_err(|e| tube::Error::msg(format!("清仓失败: {}", e)))?;
        } else {
            let mv = sell_price * remaining as f64;
            let pnl = (sell_price - avg_cost) * remaining as f64;
            let pnl_pct = if avg_cost > 0.0 {
                (sell_price - avg_cost) / avg_cost * 100.0
            } else {
                0.0
            };

            let update_sql = "UPDATE portfolio_positions SET \
                 quantity = :qty, avg_cost = :avg_cost, current_price = :price, market_value = :mv, \
                 unrealized_pnl = :pnl, unrealized_pnl_pct = :pnl_pct, \
                 snapshot_date = NOW(), modify_time = NOW() WHERE id = :id";
            execute(
                update_sql,
                vec![
                    ("qty".to_string(), Value::from(remaining)),
                    ("avg_cost".to_string(), Value::from(avg_cost)),
                    ("price".to_string(), Value::from(sell_price)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(pnl)),
                    ("pnl_pct".to_string(), Value::from(pnl_pct)),
                    ("id".to_string(), Value::from(pos_id)),
                ],
                &connector,
            ).map_err(|e| tube::Error::msg(format!("更新持仓失败: {}", e)))?;
        }

        Ok(value!({"action": "sell", "code": code, "price": sell_price, "quantity": sell_qty}))
    }

    async fn summary(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let (sql, p) = if account_id > 0 {
            ("SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct \
              FROM portfolio_positions WHERE account_id = :aid AND status = 1".to_string(),
             vec![("aid".to_string(), Value::from(account_id))])
        } else {
            ("SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct \
              FROM portfolio_positions WHERE status = 1".to_string(),
             vec![])
        };

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        let mut total_value = 0.0_f64;
        let mut total_cost = 0.0_f64;
        let mut total_pnl = 0.0_f64;
        let mut positions_detail = Vec::new();

        let real = Real::new();

        for row in &rows {
            let code = row_get_string(row, "stockCode");
            let qty: i64 = row_get_f64(row, "quantity") as i64;
            let avg_cost: f64 = row_get_f64(row, "avgCost");
            let stock_name = row_get_string(row, "stockName");

            let pure_code = code.trim_start_matches("sh")
                .trim_start_matches("sz")
                .trim_start_matches("bj")
                .trim_start_matches("SH")
                .trim_start_matches("SZ")
                .trim_start_matches("BJ");
            let prefix = utils::market_prefix(pure_code);
            let padded = format!("{:0>6}", pure_code);
            let full_code = format!("{}{}", prefix, padded);
            let db_current_price: f64 = row_get_f64(row, "currentPrice");

            let current_price = match real.get_price(&full_code).await {
                Ok(v) => {
                    let p = v.get("close")
                        .or_else(|| v.get("current_price"))
                        .or_else(|| v.get("price"))
                        .and_then(|val| val.as_f64())
                        .unwrap_or(db_current_price);
                    if p > 0.0 { p } else { db_current_price }
                }
                Err(e) => {
                    log!("portfolio summary get_price({}) error: {}", full_code, e);
                    db_current_price
                }
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
            "totalValue": total_value,
            "totalCost": total_cost,
            "totalPnl": total_pnl,
            "totalPnlPct": total_pnl_pct,
            "positionCount": positions_detail.len() as i64,
            "positions": positions_detail,
        }))
    }

    async fn positions(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let (sql, p) = if account_id > 0 {
            ("SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct, \
              snapshot_date, status, create_time, modify_time \
              FROM portfolio_positions WHERE account_id = :aid AND status = 1".to_string(),
             vec![("aid".to_string(), Value::from(account_id))])
        } else {
            ("SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct, \
              snapshot_date, status, create_time, modify_time \
              FROM portfolio_positions WHERE status = 1".to_string(),
             vec![])
        };

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        let real = Real::new();
        let mut results = Vec::new();

        for row in &rows {
            let code = row_get_string(row, "stockCode");
            let stock_name = row_get_string(row, "stockName");
            let qty: i64 = row_get_f64(row, "quantity") as i64;
            let avg_cost: f64 = row_get_f64(row, "avgCost");
            let db_current_price: f64 = row_get_f64(row, "currentPrice");

            let pure_code = code.trim_start_matches("sh")
                .trim_start_matches("sz")
                .trim_start_matches("bj")
                .trim_start_matches("SH")
                .trim_start_matches("SZ")
                .trim_start_matches("BJ");
            let prefix = utils::market_prefix(pure_code);
            let padded = format!("{:0>6}", pure_code);
            let full_code = format!("{}{}", prefix, padded);

            let current_price = match real.get_price(&full_code).await {
                Ok(v) => {
                    let p = v.get("close")
                        .or_else(|| v.get("current_price"))
                        .or_else(|| v.get("price"))
                        .and_then(|val| val.as_f64())
                        .unwrap_or(db_current_price);
                    if p > 0.0 { p } else { db_current_price }
                }
                Err(e) => {
                    log!("portfolio positions get_price({}) error: {}", full_code, e);
                    db_current_price
                }
            };

            let mv = current_price * qty as f64;
            let cost = avg_cost * qty as f64;
            let pnl = mv - cost;
            let pnl_pct = if cost > 0.0 { pnl / cost * 100.0 } else { 0.0 };

            results.push(value!({
                "id": row_get_i64(row, "id"),
                "accountId": row_get_i64(row, "accountId"),
                "stockCode": code,
                "stockName": stock_name,
                "quantity": qty,
                "avgCost": avg_cost,
                "currentPrice": current_price,
                "marketValue": mv,
                "unrealizedPnl": pnl,
                "unrealizedPnlPct": pnl_pct,
                "snapshotDate": row_get_string(row, "snapshotDate"),
                "status": row_get_i64(row, "status"),
            }));
        }

        Ok(Value::Array(results))
    }

    async fn trades(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = TradeTable::new(&self.request);
        let results = table.query_trades(account_id, limit)?;
        Ok(Value::Array(results))
    }

    async fn snapshot(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let acct_table = AccountTable::new(&self.request);
        let initial_capital = acct_table.get_initial_capital(account_id)?;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let pos_sql = "SELECT SUM(market_value) as mv, SUM(unrealized_pnl) as pnl, \
             SUM(cost_basis) as cost FROM portfolio_positions WHERE account_id = :aid AND status = 1";
        let pos_rows = query_rows(
            pos_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询持仓汇总失败: {}", e)))?;

        let market_value: f64 = pos_rows
            .first()
            .map(|r| row_get_f64(r, "mv"))
            .unwrap_or(0.0);
        let unrealized_pnl: f64 = pos_rows
            .first()
            .map(|r| row_get_f64(r, "pnl"))
            .unwrap_or(0.0);
        let total_cost: f64 = pos_rows
            .first()
            .map(|r| row_get_f64(r, "cost"))
            .unwrap_or(0.0);

        let trade_table = TradeTable::new(&self.request);
        let realized_pnl = trade_table.query_realized_pnl(account_id)?;

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

        let snap_table = DailySnapshotTable::new(&self.request);
        let prev_equity = snap_table.query_prev_equity(account_id, initial_capital)?;

        let daily_pnl = total_equity - prev_equity;
        let daily_pnl_pct = if prev_equity > 0.0 {
            daily_pnl / prev_equity * 100.0
        } else {
            0.0
        };

        snap_table.insert_snapshot(
            account_id, total_equity, cash_balance, market_value,
            daily_pnl, daily_pnl_pct, total_pnl, total_pnl_pct,
        )?;

        Ok(value!({
            "accountId": account_id,
            "totalEquity": total_equity,
            "cashBalance": cash_balance,
            "marketValue": market_value,
            "unrealizedPnl": unrealized_pnl,
            "dailyPnl": daily_pnl,
            "dailyPnlPct": daily_pnl_pct,
            "totalPnl": total_pnl,
            "totalPnlPct": total_pnl_pct,
        }))
    }

    async fn cash_ledger(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = CashLedgerTable::new(&self.request);
        let results = table.query_cash_ledger(account_id, limit)?;
        Ok(Value::Array(results))
    }

    async fn cash_add(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let direction = utils::param_string(params, "direction");
        if direction != "in" && direction != "out" {
            return Err(tube::Error::msg("direction 必须为 in 或 out".to_string()));
        }

        let amount = params
            .get("amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if amount <= 0.0 {
            return Err(tube::Error::msg("请提供有效金额".to_string()));
        }

        let note = utils::param_string(params, "note");

        let table = CashLedgerTable::new(&self.request);
        table.insert_cash_event(account_id, &direction, amount, &note)?;

        Ok(value!({"accountId": account_id, "direction": direction, "amount": amount}))
    }

    async fn corporate_actions(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = CorporateActionTable::new(&self.request);
        let results = table.query_corporate_actions(account_id, limit)?;
        Ok(Value::Array(results))
    }

    async fn corporate_action_add(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let symbol = utils::param_string(params, "symbol");
        if symbol.is_empty() {
            return Err(tube::Error::msg("请提供股票代码".to_string()));
        }

        let action_type = utils::param_string(params, "actionType");
        if action_type.is_empty() {
            return Err(tube::Error::msg("请提供行动类型".to_string()));
        }

        let cash_dividend_per_share = params
            .get("cashDividendPerShare")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let split_ratio = utils::param_string(params, "splitRatio");
        let note = utils::param_string(params, "note");

        let table = CorporateActionTable::new(&self.request);
        table.insert_corporate_action(
            account_id, &symbol, &action_type,
            cash_dividend_per_share, &split_ratio, &note,
        )?;

        Ok(value!({"accountId": account_id, "symbol": symbol, "actionType": action_type}))
    }

    async fn lots(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let table = PositionLotTable::new(&self.request);
        let results = table.query_lots(account_id)?;
        Ok(Value::Array(results))
    }

    async fn fx_rates(&self) -> Result<Value> {
        let params = self.params();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = FxRateTable::new(&self.request);
        let results = table.query_fx_rates(limit)?;
        Ok(Value::Array(results))
    }

    async fn fx_rate_update(&self) -> Result<Value> {
        let params = self.params();
        let from_currency = utils::param_string(params, "fromCurrency");
        if from_currency.is_empty() {
            return Err(tube::Error::msg("请提供源货币".to_string()));
        }

        let to_currency = utils::param_string(params, "toCurrency");
        if to_currency.is_empty() {
            return Err(tube::Error::msg("请提供目标货币".to_string()));
        }

        let rate = params
            .get("rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if rate <= 0.0 {
            return Err(tube::Error::msg("请提供有效汇率".to_string()));
        }

        let source = utils::param_string(params, "source");

        let table = FxRateTable::new(&self.request);
        table.upsert_fx_rate(&from_currency, &to_currency, rate, &source)?;

        Ok(value!({"fromCurrency": from_currency, "toCurrency": to_currency, "rate": rate}))
    }

    async fn risk_analysis(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT p.stock_code, p.market_value, p.unrealized_pnl_pct \
             FROM portfolio_positions WHERE account_id = :aid AND status = 1";
        let rows = query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "positionConcentration": 0.0,
                "sectorBreakdown": [],
                "maxSingleLoss": 0.0,
                "positionCount": 0,
            }));
        }

        let codes: Vec<String> = rows.iter().map(|r| row_get_string(r, "stockCode")).collect();
        let mut sector_cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let em = qta_crawler::EastMoney::new();
        if let Ok(spot_data) = em.stock_zh_a_spot().await {
            for item in &spot_data {
                let code = item.get("代码").and_then(|v| v.as_str()).unwrap_or_default();
                let industry = item.get("所处行业").and_then(|v| v.as_str()).unwrap_or_default();
                if !code.is_empty() && !industry.is_empty() && codes.contains(&code) {
                    sector_cache.insert(code, industry);
                }
            }
        }

        let mut total_mv = 0.0_f64;
        let mut max_mv = 0.0_f64;
        let mut max_loss_pct = 0.0_f64;
        let mut sector_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for row in &rows {
            let code = row_get_string(row, "stockCode");
            let mv: f64 = row_get_f64(row, "marketValue");
            let pnl_pct: f64 = row_get_f64(row, "unrealizedPnlPct");

            total_mv += mv;
            if mv > max_mv {
                max_mv = mv;
            }
            if pnl_pct < max_loss_pct {
                max_loss_pct = pnl_pct;
            }

            let sector = sector_cache.get(&code)
                .map(|s| s.as_str())
                .unwrap_or("其他");

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
            "positionConcentration": position_concentration,
            "sectorBreakdown": sector_breakdown,
            "maxSingleLoss": max_loss_pct,
            "positionCount": rows.len() as i64,
            "totalMarketValue": total_mv,
        }))
    }

    async fn import_trades(&self) -> Result<Value> {
        let params = self.params();
        let trades = params
            .get("trades")
            .ok_or_else(|| tube::Error::msg("请提供trades数组".to_string()))?;

        let trades_arr = trades
            .as_array()
            .ok_or_else(|| tube::Error::msg("trades必须为数组".to_string()))?;

        if trades_arr.is_empty() {
            return Err(tube::Error::msg("trades数组不能为空".to_string()));
        }

        let trade_table = TradeTable::new(&self.request);
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

            match trade_table.import_one_trade(
                account_id, &code, &name, &direction,
                price, quantity, commission, &remark,
            ) {
                Ok(_) => success_count += 1,
                Err(e) => {
                    fail_count += 1;
                    errors.push(value!({"index": idx as i64, "error": format!("插入失败: {}", e)}));
                }
            }
        }

        Ok(value!({
            "total": trades_arr.len() as i64,
            "success": success_count,
            "failed": fail_count,
            "errors": errors,
        }))
    }
}
