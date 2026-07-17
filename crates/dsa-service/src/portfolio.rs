use ai_llm_kit::LlmFactory;
use deck::sqlite::DataTable;
use deck::QueryExecutor;
use deck::TableService;
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

use qta_crawler::Real;
use tube::{Result, Value};
use tube_web::RequestParameter;

/// 从 JSON Value 中提取 f64，兼容数字和字符串类型
fn json_f64(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn extract_json_array(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with('[') {
        return trimmed.to_string();
    }
    if let Some(start) = trimmed.find('[') {
        let mut depth = 0i32;
        for (i, ch) in trimmed[start..].char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return trimmed[start..start + i + 1].to_string();
                    }
                }
                _ => {}
            }
        }
    }
    trimmed.to_string()
}

/// 从 get_price 返回的行情数据中提取当前价格
fn extract_price(v: &Value, fallback: f64) -> f64 {
    let p = v
        .get("close")
        .or_else(|| v.get("current_price"))
        .or_else(|| v.get("price"))
        .and_then(|val| json_f64(val))
        .unwrap_or(fallback);
    if p > 0.0 { p } else { fallback }
}

struct TradeTable {
    request: RequestParameter,
}

impl DataTable<PortfolioTradeModel> for TradeTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioTradeModel> for TradeTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl TradeTable {
    fn new(param: &RequestParameter) -> Self {
        TradeTable {
            request: param.clone(),
        }
    }

    fn query_trades(&self, account_id: i64, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let (sql, p) = if account_id > 0 {
            (
                "SELECT id, account_id, stock_code, stock_name, direction, price, quantity, \
              trade_date, commission, trade_currency, dedup_hash, remark, status, create_time \
              FROM portfolio_trades WHERE account_id = :aid AND status = 1 \
              ORDER BY trade_date DESC, create_time DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("aid".to_string(), Value::from(account_id)),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        } else {
            (
                "SELECT id, account_id, stock_code, stock_name, direction, price, quantity, \
              trade_date, commission, trade_currency, dedup_hash, remark, status, create_time \
              FROM portfolio_trades WHERE status = 1 \
              ORDER BY trade_date DESC, create_time DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        };
        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询交易记录失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_trade(
        &self,
        account_id: i64,
        code: &str,
        name: &str,
        direction: &str,
        price: f64,
        quantity: i64,
        commission: f64,
        remark: &str,
        trade_date: Option<chrono::NaiveDateTime>,
    ) -> Result<Value> {
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
        self.insert()
            .data(&data)
            .execute()
            .map_err(|e| tube::Error::msg(format!("插入交易记录失败: {}", e)))
    }

    fn query_cash_flows(&self, account_id: i64) -> Result<(f64, f64)> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT \
             COALESCE(SUM(CASE WHEN direction = 'buy' THEN price * quantity + commission ELSE 0 END), 0) as buy_outflow, \
             COALESCE(SUM(CASE WHEN direction = 'sell' THEN price * quantity - commission ELSE 0 END), 0) as sell_inflow \
             FROM portfolio_trades WHERE account_id = :aid AND status = 1";
        let rows = query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询交易汇总失败: {}", e)))?;
        let buy_outflow = rows.first().map(|r| row_get_f64(r, "buyOutflow")).unwrap_or(0.0);
        let sell_inflow = rows.first().map(|r| row_get_f64(r, "sellInflow")).unwrap_or(0.0);
        Ok((buy_outflow, sell_inflow))
    }

    fn import_one_trade(
        &self,
        account_id: i64,
        code: &str,
        name: &str,
        direction: &str,
        price: f64,
        quantity: i64,
        commission: f64,
        remark: &str,
    ) -> Result<Value> {
        self.insert_trade(
            account_id, code, name, direction, price, quantity, commission, remark, None,
        )
    }
}

struct AccountTable {
    request: RequestParameter,
}

impl DataTable<PortfolioAccountModel> for AccountTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioAccountModel> for AccountTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl AccountTable {
    fn new(param: &RequestParameter) -> Self {
        AccountTable {
            request: param.clone(),
        }
    }

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
        )
        .map_err(|e| tube::Error::msg(format!("查询账户失败: {}", e)))?;
        Ok(rows
            .first()
            .map(|r| row_get_f64(r, "initialCapital"))
            .unwrap_or(0.0))
    }
}

struct CashLedgerTable {
    request: RequestParameter,
}

impl DataTable<PortfolioCashLedgerModel> for CashLedgerTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioCashLedgerModel> for CashLedgerTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl CashLedgerTable {
    fn new(param: &RequestParameter) -> Self {
        CashLedgerTable {
            request: param.clone(),
        }
    }

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
        )
        .map_err(|e| tube::Error::msg(format!("查询现金流水失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_cash_event(
        &self,
        account_id: i64,
        direction: &str,
        amount: f64,
        note: &str,
    ) -> Result<Value> {
        let data = value!({
            "account_id": account_id,
            "event_date": chrono::Local::now().naive_local(),
            "direction": direction,
            "amount": amount,
            "base_currency": "CNY",
            "note": note,
            "create_time": chrono::Local::now().naive_local(),
        });
        self.insert()
            .data(&data)
            .execute()
            .map_err(|e| tube::Error::msg(format!("插入现金流水失败: {}", e)))
    }
}

struct CorporateActionTable {
    request: RequestParameter,
}

impl DataTable<PortfolioCorporateActionModel> for CorporateActionTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioCorporateActionModel> for CorporateActionTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl CorporateActionTable {
    fn new(param: &RequestParameter) -> Self {
        CorporateActionTable {
            request: param.clone(),
        }
    }

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
        )
        .map_err(|e| tube::Error::msg(format!("查询公司行动失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_corporate_action(
        &self,
        account_id: i64,
        symbol: &str,
        action_type: &str,
        cash_dividend_per_share: f64,
        split_ratio: &str,
        note: &str,
    ) -> Result<Value> {
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
        self.insert()
            .data(&data)
            .execute()
            .map_err(|e| tube::Error::msg(format!("插入公司行动失败: {}", e)))
    }
}

struct DailySnapshotTable {
    request: RequestParameter,
}

impl DataTable<PortfolioDailySnapshotModel> for DailySnapshotTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioDailySnapshotModel> for DailySnapshotTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl DailySnapshotTable {
    fn new(param: &RequestParameter) -> Self {
        DailySnapshotTable {
            request: param.clone(),
        }
    }

    fn insert_snapshot(
        &self,
        account_id: i64,
        total_equity: f64,
        cash_balance: f64,
        market_value: f64,
        daily_pnl: f64,
        daily_pnl_pct: f64,
        total_pnl: f64,
        total_pnl_pct: f64,
    ) -> Result<Value> {
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
        self.insert()
            .data(&data)
            .execute()
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
        )
        .unwrap_or_default();
        Ok(rows
            .first()
            .map(|r| row_get_f64(r, "totalEquity"))
            .unwrap_or(fallback))
    }
}

struct FxRateTable {
    request: RequestParameter,
}

impl DataTable<PortfolioFxRateModel> for FxRateTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioFxRateModel> for FxRateTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl FxRateTable {
    fn new(param: &RequestParameter) -> Self {
        FxRateTable {
            request: param.clone(),
        }
    }

    fn query_fx_rates(&self, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT * FROM portfolio_fx_rates ORDER BY rate_date DESC LIMIT :limit";
        let rows = query_rows(
            sql,
            vec![("limit".to_string(), Value::from(limit))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询汇率失败: {}", e)))?;
        Ok(rows)
    }

    fn upsert_fx_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        rate: f64,
        source: &str,
    ) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
        let now_expr = if is_sqlite {
            "datetime('now')"
        } else {
            "NOW()"
        };
        let upsert_clause = if is_sqlite {
            "rate = excluded.rate, source = excluded.source, is_stale = 0, updated_time = datetime('now')"
        } else {
            "rate = VALUES(rate), source = VALUES(source), is_stale = 0, updated_time = NOW()"
        };
        let sql = format!(
            "INSERT INTO portfolio_fx_rates \
             (from_currency, to_currency, rate_date, rate, source, is_stale, updated_time) \
             VALUES (:from, :to, {}, :rate, :src, 0, {}) \
             ON DUPLICATE KEY UPDATE {}",
            now_expr, now_expr, upsert_clause
        );
        execute(
            &sql,
            vec![
                ("from".to_string(), Value::from(from_currency)),
                ("to".to_string(), Value::from(to_currency)),
                ("rate".to_string(), Value::from(rate)),
                ("src".to_string(), Value::from(source)),
            ],
            &connector,
        )
        .map(|n| tube::Value::from(n as i64))
        .map_err(|e| tube::Error::msg(format!("更新汇率失败: {}", e)))
    }
}

struct PositionLotTable {
    request: RequestParameter,
}

impl DataTable<PortfolioPositionLotModel> for PositionLotTable {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}
impl TableService<PortfolioPositionLotModel> for PositionLotTable {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}
impl PositionLotTable {
    fn new(param: &RequestParameter) -> Self {
        PositionLotTable {
            request: param.clone(),
        }
    }

    fn query_lots(&self, account_id: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT * FROM portfolio_position_lots \
             WHERE account_id = :aid AND remaining_quantity > 0 ORDER BY open_date ASC";
        let rows = query_rows(
            sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询FIFO批次失败: {}", e)))?;
        Ok(rows)
    }
}

pub struct Portfolio {
    request: RequestParameter,
}

impl DataTable<PortfolioPositionModel> for Portfolio {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}

impl TableService<PortfolioPositionModel> for Portfolio {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}

impl Portfolio {
    pub fn new(param: &RequestParameter) -> Self {
        Portfolio {
            request: param.clone(),
        }
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
            "ocr_import" => self.ocr_import().await,
            "rebuild_positions" => self.rebuild_positions().await,
            "edit_trade" => self.edit_trade().await,
            "delete_trade" => self.delete_trade().await,
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

        let price = params.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
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

        let trade_date = params
            .get("tradeDate")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .or_else(|| {
                        chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                            .ok()
                            .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    })
            });

        let trade_table = TradeTable::new(&self.request);
        trade_table.insert_trade(
            account_id, &code, &name, "buy", price, quantity, commission, &remark, trade_date,
        )?;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let check_sql = "SELECT id, quantity, avg_cost, realized_pnl, total_commission, total_buy_amount, total_sell_amount FROM portfolio_positions \
             WHERE account_id = :aid AND stock_code = :code AND status = 1 LIMIT 1";
        let existing = query_rows(
            check_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        let buy_amount = price * quantity as f64;

        if existing.is_empty() {
            let avg = if quantity > 0 { (buy_amount + commission) / quantity as f64 } else { price };
            let mv = price * quantity as f64;
            let cost_basis = avg * quantity as f64;
            let pnl = mv - cost_basis;
            let pnl_pct = if cost_basis > 0.0 { pnl / cost_basis * 100.0 } else { 0.0 };
            let data = value!({
                "account_id": account_id,
                "stock_code": code.clone(),
                "stock_name": name,
                "quantity": quantity,
                "avg_cost": avg,
                "current_price": price,
                "market_value": mv,
                "unrealized_pnl": pnl,
                "unrealized_pnl_pct": pnl_pct,
                "realized_pnl": 0.0_f64,
                "total_commission": commission,
                "total_buy_amount": buy_amount,
                "total_sell_amount": 0.0_f64,
                "snapshot_date": chrono::Local::now().naive_local(),
                "status": 1,
                "create_time": chrono::Local::now().naive_local(),
                "modify_time": chrono::Local::now().naive_local(),
            });
            self.insert()
                .data(&data)
                .execute()
                .map_err(|e| tube::Error::msg(format!("创建持仓失败: {}", e)))?;
        } else {
            let row = &existing[0];
            let pos_id: i64 = row_get_i64(row, "id");
            let old_qty: i64 = row_get_f64(row, "quantity") as i64;
            let old_realized: f64 = row_get_f64(row, "realizedPnl");
            let old_total_comm: f64 = row_get_f64(row, "totalCommission");
            let old_buy_amount: f64 = row_get_f64(row, "totalBuyAmount");
            let old_sell_amount: f64 = row_get_f64(row, "totalSellAmount");

            let new_qty = old_qty + quantity;
            let new_buy_amount = old_buy_amount + buy_amount;
            let new_total_comm = old_total_comm + commission;
            let new_avg = if new_qty > 0 {
                (new_buy_amount - old_sell_amount + new_total_comm) / new_qty as f64
            } else {
                price
            };            let mv = price * new_qty as f64;
            let cost_basis = new_avg * new_qty as f64;
            let pnl = mv - cost_basis;
            let pnl_pct = if cost_basis > 0.0 { pnl / cost_basis * 100.0 } else { 0.0 };

            let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
            let now_expr = if is_sqlite {
                "datetime('now')"
            } else {
                "NOW()"
            };
            let update_sql = format!(
                "UPDATE portfolio_positions SET \
                 quantity = :qty, avg_cost = :avg_cost, current_price = :price, \
                 market_value = :mv, unrealized_pnl = :pnl, unrealized_pnl_pct = :pnl_pct, \
                 realized_pnl = :realized_pnl, total_commission = :total_commission, \
                 total_buy_amount = :total_buy_amount, total_sell_amount = :total_sell_amount, \
                 snapshot_date = {}, modify_time = {} \
                 WHERE id = :id",
                now_expr, now_expr
            );
            execute(
                &update_sql,
                vec![
                    ("qty".to_string(), Value::from(new_qty)),
                    ("avg_cost".to_string(), Value::from(new_avg)),
                    ("price".to_string(), Value::from(price)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(pnl)),
                    ("pnl_pct".to_string(), Value::from(pnl_pct)),
                    ("realized_pnl".to_string(), Value::from(old_realized)),
                    ("total_commission".to_string(), Value::from(new_total_comm)),
                    ("total_buy_amount".to_string(), Value::from(new_buy_amount)),
                    ("total_sell_amount".to_string(), Value::from(old_sell_amount)),
                    ("id".to_string(), Value::from(pos_id)),
                ],
                &connector,
            )
            .map_err(|e| tube::Error::msg(format!("更新持仓失败: {}", e)))?;
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

        let price = params.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let quantity = params
            .get("quantity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let commission = params
            .get("commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let remark = utils::param_string(params, "remark");

        let trade_date = params
            .get("tradeDate")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .or_else(|| {
                        chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                            .ok()
                            .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    })
            });

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let check_sql = "SELECT id, quantity, avg_cost, stock_name, realized_pnl, total_commission, total_buy_amount, total_sell_amount FROM portfolio_positions \
             WHERE account_id = :aid AND stock_code = :code AND status = 1 LIMIT 1";
        let existing = query_rows(
            check_sql,
            vec![
                ("aid".to_string(), Value::from(account_id)),
                ("code".to_string(), Value::from(code.as_str())),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        if existing.is_empty() {
            return Err(tube::Error::msg(format!("无持仓: {}", code)));
        }

        let row = &existing[0];
        let pos_id: i64 = row_get_i64(row, "id");
        let old_qty: i64 = row_get_f64(row, "quantity") as i64;
        let avg_cost: f64 = row_get_f64(row, "avgCost");
        let stock_name = row_get_string(row, "stockName");
        let old_realized: f64 = row_get_f64(row, "realizedPnl");
        let old_total_comm: f64 = row_get_f64(row, "totalCommission");
        let old_buy_amount: f64 = row_get_f64(row, "totalBuyAmount");
        let old_sell_amount: f64 = row_get_f64(row, "totalSellAmount");

        let sell_qty = if quantity <= 0 || quantity >= old_qty {
            old_qty
        } else {
            quantity
        };

        let sell_price = if price <= 0.0 { avg_cost } else { price };

        let trade_table = TradeTable::new(&self.request);
        trade_table.insert_trade(
            account_id,
            &code,
            &stock_name,
            "sell",
            sell_price,
            sell_qty,
            commission,
            &remark,
            trade_date,
        )?;

        let sell_realized = (sell_price - avg_cost) * sell_qty as f64 - commission;
        let new_realized = old_realized + sell_realized;
        let new_total_comm = old_total_comm + commission;
        let new_sell_amount = old_sell_amount + sell_price * sell_qty as f64;

        let remaining = old_qty - sell_qty;
        if remaining <= 0 {
            let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
            let now_expr = if is_sqlite {
                "datetime('now')"
            } else {
                "NOW()"
            };
            let update_sql = format!(
                "UPDATE portfolio_positions SET quantity = 0, status = 0, \
                 realized_pnl = :realized_pnl, total_commission = :total_commission, \
                 total_buy_amount = :total_buy_amount, total_sell_amount = :total_sell_amount, \
                 modify_time = {} WHERE id = :id",
                now_expr
            );
            execute(
                &update_sql,
                vec![
                    ("realized_pnl".to_string(), Value::from(new_realized)),
                    ("total_commission".to_string(), Value::from(new_total_comm)),
                    ("total_buy_amount".to_string(), Value::from(old_buy_amount)),
                    ("total_sell_amount".to_string(), Value::from(new_sell_amount)),
                    ("id".to_string(), Value::from(pos_id)),
                ],
                &connector,
            )
            .map_err(|e| tube::Error::msg(format!("清仓失败: {}", e)))?;
        } else {
            let new_avg = if remaining > 0 {
                (old_buy_amount - new_sell_amount + new_total_comm) / remaining as f64
            } else {
                sell_price
            };
            let mv = sell_price * remaining as f64;
            let cost_basis = new_avg * remaining as f64;
            let pnl = mv - cost_basis;
            let pnl_pct = if cost_basis > 0.0 { pnl / cost_basis * 100.0 } else { 0.0 };

            let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
            let now_expr = if is_sqlite {
                "datetime('now')"
            } else {
                "NOW()"
            };
            let update_sql = format!(
                "UPDATE portfolio_positions SET \
                 quantity = :qty, avg_cost = :avg_cost, current_price = :price, market_value = :mv, \
                 unrealized_pnl = :pnl, unrealized_pnl_pct = :pnl_pct, \
                 realized_pnl = :realized_pnl, total_commission = :total_commission, \
                 total_buy_amount = :total_buy_amount, total_sell_amount = :total_sell_amount, \
                 snapshot_date = {}, modify_time = {} WHERE id = :id",
                now_expr, now_expr
            );
            execute(
                &update_sql,
                vec![
                    ("qty".to_string(), Value::from(remaining)),
                    ("avg_cost".to_string(), Value::from(new_avg)),
                    ("price".to_string(), Value::from(sell_price)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(pnl)),
                    ("pnl_pct".to_string(), Value::from(pnl_pct)),
                    ("realized_pnl".to_string(), Value::from(new_realized)),
                    ("total_commission".to_string(), Value::from(new_total_comm)),
                    ("total_buy_amount".to_string(), Value::from(old_buy_amount)),
                    ("total_sell_amount".to_string(), Value::from(new_sell_amount)),
                    ("id".to_string(), Value::from(pos_id)),
                ],
                &connector,
            )
            .map_err(|e| tube::Error::msg(format!("更新持仓失败: {}", e)))?;
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
            (
                "SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct, \
              realized_pnl, total_commission, total_buy_amount, total_sell_amount \
              FROM portfolio_positions WHERE account_id = :aid AND status = 1 ORDER BY modify_time DESC"
                    .to_string(),
                vec![("aid".to_string(), Value::from(account_id))],
            )
        } else {
            (
                "SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct, \
              realized_pnl, total_commission, total_buy_amount, total_sell_amount \
              FROM portfolio_positions WHERE status = 1 ORDER BY modify_time DESC"
                    .to_string(),
                vec![],
            )
        };

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        let mut total_value = 0.0_f64;
        let mut total_cost = 0.0_f64;
        let mut total_unrealized = 0.0_f64;
        let mut total_realized = 0.0_f64;
        let mut positions_detail = Vec::new();

        let real = Real::new();

        for row in &rows {
            let code = row_get_string(row, "stockCode");
            let qty: i64 = row_get_f64(row, "quantity") as i64;
            let avg_cost: f64 = row_get_f64(row, "avgCost");
            let stock_name = row_get_string(row, "stockName");

            let pure_code = code
                .trim_start_matches("sh")
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
                Ok(v) => extract_price(&v, db_current_price),
                Err(e) => {
                    log!("portfolio summary get_price({}) error: {}", full_code, e);
                    db_current_price
                }
            };

            let mv = current_price * qty as f64;
            let cost = avg_cost * qty as f64;
            let unrealized = mv - cost;
            let realized: f64 = row_get_f64(row, "realizedPnl");

            total_value += mv;
            total_cost += cost;
            total_unrealized += unrealized;
            total_realized += realized;

            let pnl_pct = if cost > 0.0 { unrealized / cost * 100.0 } else { 0.0 };

            positions_detail.push(value!({
                "code": code,
                "name": stock_name,
                "quantity": qty,
                "avgCost": avg_cost,
                "currentPrice": current_price,
                "marketValue": mv,
                "unrealizedPnl": unrealized,
                "unrealizedPnlPct": pnl_pct,
                "realizedPnl": realized,
                "totalCommission": row_get_f64(row, "totalCommission"),
                "totalBuyAmount": row_get_f64(row, "totalBuyAmount"),
                "totalSellAmount": row_get_f64(row, "totalSellAmount"),
            }));
        }

        let unrealized_pnl = total_unrealized;
        let unrealized_pnl_pct = if total_cost > 0.0 {
            total_unrealized / total_cost * 100.0
        } else {
            0.0
        };

        let mut cash_balance = 0.0_f64;
        let mut initial_capital = 0.0_f64;
        if account_id > 0 {
            let acct_table = AccountTable::new(&self.request);
            if let Ok(ic) = acct_table.get_initial_capital(account_id) {
                initial_capital = ic;
                let trade_table = TradeTable::new(&self.request);
                if let Ok((buy_outflow, sell_inflow)) = trade_table.query_cash_flows(account_id) {
                    cash_balance = initial_capital - buy_outflow + sell_inflow;
                }
            }
        }

        let total_equity = total_value + cash_balance;
        let total_pnl_all = if initial_capital > 0.0 { total_equity - initial_capital } else { total_unrealized + total_realized };
        let total_pnl_pct_all = if initial_capital > 0.0 {
            total_pnl_all / initial_capital * 100.0
        } else {
            unrealized_pnl_pct
        };

        Ok(value!({
            "totalValue": total_value,
            "totalCost": total_cost,
            "totalPnl": total_pnl_all,
            "totalPnlPct": total_pnl_pct_all,
            "totalEquity": total_equity,
            "cashBalance": cash_balance,
            "unrealizedPnl": unrealized_pnl,
            "unrealizedPnlPct": unrealized_pnl_pct,
            "realizedPnl": total_realized,
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
            (
                "SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct, \
              realized_pnl, total_commission, total_buy_amount, total_sell_amount, \
              snapshot_date, status, create_time, modify_time \
              FROM portfolio_positions WHERE account_id = :aid AND status = 1 ORDER BY modify_time DESC"
                    .to_string(),
                vec![("aid".to_string(), Value::from(account_id))],
            )
        } else {
            (
                "SELECT id, account_id, stock_code, stock_name, quantity, avg_cost, \
              current_price, market_value, unrealized_pnl, unrealized_pnl_pct, \
              realized_pnl, total_commission, total_buy_amount, total_sell_amount, \
              snapshot_date, status, create_time, modify_time \
              FROM portfolio_positions WHERE status = 1 ORDER BY modify_time DESC"
                    .to_string(),
                vec![],
            )
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

            let pure_code = code
                .trim_start_matches("sh")
                .trim_start_matches("sz")
                .trim_start_matches("bj")
                .trim_start_matches("SH")
                .trim_start_matches("SZ")
                .trim_start_matches("BJ");
            let prefix = utils::market_prefix(pure_code);
            let padded = format!("{:0>6}", pure_code);
            let full_code = format!("{}{}", prefix, padded);

            let current_price = match real.get_price(&full_code).await {
                Ok(v) => extract_price(&v, db_current_price),
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
                "realizedPnl": row_get_f64(row, "realizedPnl"),
                "totalCommission": row_get_f64(row, "totalCommission"),
                "totalBuyAmount": row_get_f64(row, "totalBuyAmount"),
                "totalSellAmount": row_get_f64(row, "totalSellAmount"),
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
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(50.0) as i64;

        let table = TradeTable::new(&self.request);
        let results = table.query_trades(account_id, limit)?;
        Ok(Value::Array(results))
    }

    async fn edit_trade(&self) -> Result<Value> {
        let params = self.params();
        let trade_id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if trade_id == 0 {
            return Err(tube::Error::msg("请提供交易ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let check_sql = "SELECT account_id FROM portfolio_trades WHERE id = :id AND status = 1 LIMIT 1";
        let rows = query_rows(
            check_sql,
            vec![("id".to_string(), Value::from(trade_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询交易失败: {}", e)))?;

        if rows.is_empty() {
            return Err(tube::Error::msg("交易记录不存在".to_string()));
        }

        let account_id: i64 = row_get_f64(&rows[0], "accountId") as i64;
        let code = utils::param_string(params, "code");
        let name = utils::param_string(params, "name");
        let direction = utils::param_string(params, "direction");
        let price = params.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let quantity = params.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        let commission = params.get("commission").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let remark = utils::param_string(params, "remark");

        let trade_date = params
            .get("tradeDate")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .or_else(|| {
                        chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                            .ok()
                            .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                    })
            });

        if price <= 0.0 {
            return Err(tube::Error::msg("价格必须大于0".to_string()));
        }
        if quantity <= 0 {
            return Err(tube::Error::msg("数量必须大于0".to_string()));
        }
        if direction != "buy" && direction != "sell" {
            return Err(tube::Error::msg("方向只能是buy或sell".to_string()));
        }

        let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
        let now_expr = if is_sqlite { "datetime('now')" } else { "NOW()" };
        let mut set_clauses = Vec::new();
        let mut bind_params: Vec<(String, Value)> = Vec::new();

        if !code.is_empty() {
            set_clauses.push("stock_code = :code".to_string());
            bind_params.push(("code".to_string(), Value::from(code.as_str())));
        }
        if !name.is_empty() {
            set_clauses.push("stock_name = :name".to_string());
            bind_params.push(("name".to_string(), Value::from(name.as_str())));
        }
        set_clauses.push("direction = :direction".to_string());
        bind_params.push(("direction".to_string(), Value::from(direction.as_str())));
        set_clauses.push("price = :price".to_string());
        bind_params.push(("price".to_string(), Value::from(price)));
        set_clauses.push("quantity = :quantity".to_string());
        bind_params.push(("quantity".to_string(), Value::from(quantity)));
        set_clauses.push("commission = :commission".to_string());
        bind_params.push(("commission".to_string(), Value::from(commission)));
        set_clauses.push("remark = :remark".to_string());
        bind_params.push(("remark".to_string(), Value::from(remark.as_str())));
        if trade_date.is_some() {
            set_clauses.push(format!("trade_date = :trade_date"));
            let td = trade_date.unwrap();
            if is_sqlite {
                bind_params.push(("trade_date".to_string(), Value::from(td.format("%Y-%m-%d %H:%M:%S").to_string())));
            } else {
                bind_params.push(("trade_date".to_string(), Value::from(td)));
            }
        }

        if set_clauses.is_empty() {
            return Ok(value!({"id": trade_id, "updated": false}));
        }

        let update_sql = format!(
            "UPDATE portfolio_trades SET {} WHERE id = :id",
            set_clauses.join(", ")
        );
        bind_params.push(("id".to_string(), Value::from(trade_id)));
        execute(&update_sql, bind_params, &connector)
            .map_err(|e| tube::Error::msg(format!("更新交易失败: {}", e)))?;

        self.rebuild_after_change(account_id, &connector)?;

        Ok(value!({"id": trade_id, "updated": true}))
    }

    async fn delete_trade(&self) -> Result<Value> {
        let params = self.params();
        let trade_id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if trade_id == 0 {
            return Err(tube::Error::msg("请提供交易ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let check_sql = "SELECT account_id FROM portfolio_trades WHERE id = :id AND status = 1 LIMIT 1";
        let rows = query_rows(
            check_sql,
            vec![("id".to_string(), Value::from(trade_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询交易失败: {}", e)))?;

        if rows.is_empty() {
            return Err(tube::Error::msg("交易记录不存在".to_string()));
        }

        let account_id: i64 = row_get_f64(&rows[0], "accountId") as i64;

        let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
        let now_expr = if is_sqlite { "datetime('now')" } else { "NOW()" };
        let update_sql = format!("UPDATE portfolio_trades SET status = 0 WHERE id = :id");
        execute(
            &update_sql,
            vec![("id".to_string(), Value::from(trade_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("删除交易失败: {}", e)))?;

        self.rebuild_after_change(account_id, &connector)?;

        Ok(value!({"id": trade_id, "deleted": true}))
    }

    fn rebuild_after_change(&self, account_id: i64, connector: &deck_connector::Connector) -> Result<()> {
        let trade_sql = "SELECT stock_code, stock_name, direction, price, quantity, commission, trade_date \
             FROM portfolio_trades WHERE account_id = :aid AND status = 1 ORDER BY trade_date ASC, id ASC";
        let trades = query_rows(
            trade_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询交易记录失败: {}", e)))?;

        let mut positions: std::collections::HashMap<String, (i64, f64, f64, f64, f64, String)> = std::collections::HashMap::new();

        for trade in &trades {
            let code = row_get_string(trade, "stockCode");
            let name = row_get_string(trade, "stockName");
            let direction = row_get_string(trade, "direction");
            let price: f64 = row_get_f64(trade, "price");
            let qty: i64 = row_get_f64(trade, "quantity") as i64;
            let commission: f64 = row_get_f64(trade, "commission");

            let entry = positions.entry(code.clone()).or_insert((0, 0.0, 0.0, 0.0, 0.0, name.clone()));

            if direction == "buy" {
                entry.0 += qty;
                entry.1 += price * qty as f64;
                entry.4 += commission;
                if !name.is_empty() { entry.5 = name.clone(); }
            } else if direction == "sell" {
                let current_avg = if entry.0 > 0 {
                    (entry.1 - entry.2 + entry.4) / entry.0 as f64
                } else {
                    price
                };
                let sell_realized = (price - current_avg) * qty as f64 - commission;
                entry.3 += sell_realized;
                entry.2 += price * qty as f64;
                entry.4 += commission;
                let remaining = entry.0 - qty;
                if remaining <= 0 {
                    entry.0 = 0;
                } else {
                    entry.0 = remaining;
                }
            }
        }

        let delete_sql = "DELETE FROM portfolio_positions WHERE account_id = :aid";
        execute(
            delete_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            connector,
        )
        .map_err(|e| tube::Error::msg(format!("清除旧持仓失败: {}", e)))?;

        let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
        let now_expr = if is_sqlite { "datetime('now')" } else { "NOW()" };

        for (code, (qty, buy_amount, sell_amount, realized_pnl, total_commission, name)) in &positions {
            if *qty <= 0 { continue; }
            let avg = (*buy_amount - *sell_amount + *total_commission) / *qty as f64;
            let mv = avg * *qty as f64;
            let insert_sql = format!(
                "INSERT INTO portfolio_positions \
                 (account_id, stock_code, stock_name, quantity, avg_cost, current_price, market_value, \
                  unrealized_pnl, unrealized_pnl_pct, realized_pnl, total_commission, \
                  total_buy_amount, total_sell_amount, \
                  snapshot_date, status, create_time, modify_time) \
                 VALUES (:aid, :code, :name, :qty, :avg, :avg, :mv, 0, 0, :realized_pnl, :total_commission, \
                  :total_buy_amount, :total_sell_amount, {}, 1, {}, {})",
                now_expr, now_expr, now_expr
            );
            execute(
                &insert_sql,
                vec![
                    ("aid".to_string(), Value::from(account_id)),
                    ("code".to_string(), Value::from(code.as_str())),
                    ("name".to_string(), Value::from(name.as_str())),
                    ("qty".to_string(), Value::from(*qty)),
                    ("avg".to_string(), Value::from(avg)),
                    ("mv".to_string(), Value::from(mv)),
                    ("realized_pnl".to_string(), Value::from(*realized_pnl)),
                    ("total_commission".to_string(), Value::from(*total_commission)),
                    ("total_buy_amount".to_string(), Value::from(*buy_amount)),
                    ("total_sell_amount".to_string(), Value::from(*sell_amount)),
                ],
                connector,
            )
            .map_err(|e| tube::Error::msg(format!("重建持仓失败: {}", e)))?;
        }

        Ok(())
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
        let pos_sql = "SELECT SUM(market_value) as mv, SUM(avg_cost * quantity) as cost_basis, \
             SUM(realized_pnl) as total_realized \
             FROM portfolio_positions WHERE account_id = :aid AND status = 1";
        let pos_rows = query_rows(
            pos_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询持仓汇总失败: {}", e)))?;

        let market_value: f64 = pos_rows
            .first()
            .map(|r| row_get_f64(r, "mv"))
            .unwrap_or(0.0);
        let cost_basis: f64 = pos_rows
            .first()
            .map(|r| row_get_f64(r, "costBasis"))
            .unwrap_or(0.0);
        let realized_pnl: f64 = pos_rows
            .first()
            .map(|r| row_get_f64(r, "totalRealized"))
            .unwrap_or(0.0);
        let unrealized_pnl = market_value - cost_basis;

        let trade_table = TradeTable::new(&self.request);
        let (buy_outflow, sell_inflow) = trade_table.query_cash_flows(account_id)?;

        let cash_balance = initial_capital - buy_outflow + sell_inflow;
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
            account_id,
            total_equity,
            cash_balance,
            market_value,
            daily_pnl,
            daily_pnl_pct,
            total_pnl,
            total_pnl_pct,
        )?;

        Ok(value!({
            "accountId": account_id,
            "totalEquity": total_equity,
            "cashBalance": cash_balance,
            "marketValue": market_value,
            "unrealizedPnl": unrealized_pnl,
            "realizedPnl": realized_pnl,
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

        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(50.0) as i64;

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

        let amount = params.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
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

        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(50.0) as i64;

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
            account_id,
            &symbol,
            &action_type,
            cash_dividend_per_share,
            &split_ratio,
            &note,
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
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(50.0) as i64;

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

        let rate = params.get("rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
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
        )
        .map_err(|e| tube::Error::msg(format!("查询持仓失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "positionConcentration": 0.0,
                "sectorBreakdown": [],
                "maxSingleLoss": 0.0,
                "positionCount": 0,
            }));
        }

        let codes: Vec<String> = rows
            .iter()
            .map(|r| row_get_string(r, "stockCode"))
            .collect();
        let mut sector_cache: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let em = qta_crawler::EastMoney::new();
        if let Ok(spot_data) = em.stock_zh_a_spot().await {
            for item in &spot_data {
                let code = item
                    .get("代码")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let industry = item
                    .get("所处行业")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if !code.is_empty() && !industry.is_empty() && codes.contains(&code) {
                    sector_cache.insert(code, industry);
                }
            }
        }

        let mut total_mv = 0.0_f64;
        let mut max_mv = 0.0_f64;
        let mut max_loss_pct = 0.0_f64;
        let mut sector_map: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

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

            let sector = sector_cache
                .get(&code)
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

    async fn rebuild_positions(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let trade_sql = "SELECT stock_code, stock_name, direction, price, quantity, commission, trade_date \
             FROM portfolio_trades WHERE account_id = :aid AND status = 1 ORDER BY trade_date ASC, id ASC";
        let trades = query_rows(
            trade_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询交易记录失败: {}", e)))?;

        let mut positions: std::collections::HashMap<String, (i64, f64, f64, f64, f64, String)> = std::collections::HashMap::new();

        for trade in &trades {
            let code = row_get_string(trade, "stockCode");
            let name = row_get_string(trade, "stockName");
            let direction = row_get_string(trade, "direction");
            let price: f64 = row_get_f64(trade, "price");
            let qty: i64 = row_get_f64(trade, "quantity") as i64;
            let commission: f64 = row_get_f64(trade, "commission");

            let entry = positions.entry(code.clone()).or_insert((0, 0.0, 0.0, 0.0, 0.0, name.clone()));
            // (qty, buy_amount, sell_amount, realized_pnl, total_commission, name)

            if direction == "buy" {
                entry.0 += qty;
                entry.1 += price * qty as f64;
                entry.4 += commission;
                if !name.is_empty() { entry.5 = name.clone(); }
            } else if direction == "sell" {
                let current_avg = if entry.0 > 0 {
                    (entry.1 - entry.2 + entry.4) / entry.0 as f64
                } else {
                    price
                };
                let sell_realized = (price - current_avg) * qty as f64 - commission;
                entry.3 += sell_realized;
                entry.2 += price * qty as f64;
                entry.4 += commission;
                let remaining = entry.0 - qty;
                if remaining <= 0 {
                    entry.0 = 0;
                } else {
                    entry.0 = remaining;
                }
            }
        }

        let delete_sql = "DELETE FROM portfolio_positions WHERE account_id = :aid";
        execute(
            delete_sql,
            vec![("aid".to_string(), Value::from(account_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("清除旧持仓失败: {}", e)))?;

        let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
        let now_expr = if is_sqlite { "datetime('now')" } else { "NOW()" };

        let mut rebuilt_count = 0_i64;
        for (code, (qty, buy_amount, sell_amount, realized_pnl, total_commission, name)) in &positions {
            if *qty <= 0 { continue; }
            let avg = (*buy_amount - *sell_amount + *total_commission) / *qty as f64;
            let mv = avg * *qty as f64;
            let pnl = 0.0_f64;
            let pnl_pct = 0.0_f64;
            let insert_sql = format!(
                "INSERT INTO portfolio_positions \
                 (account_id, stock_code, stock_name, quantity, avg_cost, current_price, market_value, \
                  unrealized_pnl, unrealized_pnl_pct, realized_pnl, total_commission, \
                  total_buy_amount, total_sell_amount, \
                  snapshot_date, status, create_time, modify_time) \
                 VALUES (:aid, :code, :name, :qty, :avg, :avg, :mv, :pnl, :pnl_pct, :realized_pnl, :total_commission, \
                  :total_buy_amount, :total_sell_amount, {}, 1, {}, {})",
                now_expr, now_expr, now_expr
            );
            execute(
                &insert_sql,
                vec![
                    ("aid".to_string(), Value::from(account_id)),
                    ("code".to_string(), Value::from(code.as_str())),
                    ("name".to_string(), Value::from(name.as_str())),
                    ("qty".to_string(), Value::from(*qty)),
                    ("avg".to_string(), Value::from(avg)),
                    ("mv".to_string(), Value::from(mv)),
                    ("pnl".to_string(), Value::from(pnl)),
                    ("pnl_pct".to_string(), Value::from(pnl_pct)),
                    ("realized_pnl".to_string(), Value::from(*realized_pnl)),
                    ("total_commission".to_string(), Value::from(*total_commission)),
                    ("total_buy_amount".to_string(), Value::from(*buy_amount)),
                    ("total_sell_amount".to_string(), Value::from(*sell_amount)),
                ],
                &connector,
            )
            .map_err(|e| tube::Error::msg(format!("重建持仓失败: {}", e)))?;
            rebuilt_count += 1;
        }

        Ok(value!({
            "accountId": account_id,
            "totalTrades": trades.len() as i64,
            "rebuiltPositions": rebuilt_count,
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

            let price = trade.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
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
                account_id, &code, &name, &direction, price, quantity, commission, &remark,
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

    async fn ocr_import(&self) -> Result<Value> {
        let params = self.params();
        let account_id = params
            .get("accountId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if account_id == 0 {
            return Err(tube::Error::msg("请提供账户ID".to_string()));
        }

        let image_base64 = utils::param_string(params, "image");
        if image_base64.is_empty() {
            return Err(tube::Error::msg("请提供图片数据(base64)".to_string()));
        }

        let conf = dsa_core::get_global_config();
        let vision_provider = if conf.llm.vision_provider.is_empty() {
            &conf.llm.provider
        } else {
            &conf.llm.vision_provider
        };
        let vision_model = if conf.llm.vision_model.is_empty() {
            &conf.llm.model
        } else {
            &conf.llm.vision_model
        };

        let api_key = if !conf.llm.vision_api_key.is_empty() {
            conf.llm.vision_api_key.clone()
        } else if !conf.llm.api_key.is_empty() {
            conf.llm.api_key.clone()
        } else if !conf.llm.api_key_env.is_empty() {
            std::env::var(&conf.llm.api_key_env).unwrap_or_default()
        } else {
            return Err(tube::Error::msg("LLM API Key 未配置，无法使用截图识别".to_string()));
        };

        let provider = ai_llm_kit::LlmProvider::instance(vision_provider)
            .map_err(|e| tube::Error::msg(format!("不支持的视觉模型provider: {}", e)))?;
        let llm = LlmFactory::create(provider, &api_key);

        let prompt = r#"你是一个专业的股票交易记录识别助手。请识别图片中的交易记录信息，并输出为JSON数组格式。

每条交易记录包含以下字段：
- code: 股票代码（6位数字，如 600519）
- name: 股票名称（如 贵州茅台）
- direction: 交易方向，"buy"表示买入，"sell"表示卖出
- price: 成交价格（浮点数）
- quantity: 成交数量（整数，股数）
- commission: 手续费/佣金（浮点数，如果无法识别则填0）
- tradeDate: 交易日期（格式 YYYY-MM-DD，如果无法识别则留空）

输出要求：
1. 只输出JSON数组，不要包含任何其他文字说明
2. 如果识别到多笔交易，输出多条记录
3. 如果某字段无法识别，尽量根据上下文推断，实在无法推断的留空字符串或0
4. 示例格式: [{"code":"600519","name":"贵州茅台","direction":"buy","price":1688.50,"quantity":100,"commission":5.00,"tradeDate":"2025-01-15"}]

请识别以下截图中的交易记录："#;

        let image_data = if image_base64.starts_with("data:image") {
            image_base64.clone()
        } else {
            format!("data:image/png;base64,{}", image_base64)
        };

        let body = value!({
            "model": vision_model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": prompt},
                        {"type": "image_url", "image_url": {"url": image_data}}
                    ]
                }
            ],
            "temperature": 0.1,
            "max_tokens": 4096
        });

        let response = llm.chat(&body).await.map_err(|e| {
            tube::Error::msg(format!("调用视觉模型失败: {}", e))
        })?;

        let choices = response
            .get("choices")
            .and_then(|c| c.as_array())
            .unwrap_or_default();
        let content = choices
            .first()
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        let json_str = extract_json_array(&content);
        let trades_value: Value = serde_json::from_str(&json_str)
            .map_err(|e| tube::Error::msg(format!("解析LLM返回的交易数据失败: {}, 原始内容: {}", e, &content[..content.len().min(200)])))?;

        let trades_arr = trades_value
            .as_array()
            .ok_or_else(|| tube::Error::msg("LLM返回的交易数据不是数组".to_string()))?;

        if trades_arr.is_empty() {
            return Ok(value!({
                "total": 0,
                "success": 0,
                "failed": 0,
                "rawContent": content,
                "errors": [],
            }));
        }

        let trade_table = TradeTable::new(&self.request);
        let mut success_count = 0_i64;
        let mut fail_count = 0_i64;
        let mut errors: Vec<Value> = Vec::new();

        for (idx, trade) in trades_arr.iter().enumerate() {
            let code = trade.get("code").and_then(|v| v.as_str()).unwrap_or_default();
            if code.is_empty() {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "无法识别股票代码"}));
                continue;
            }

            let direction = trade.get("direction").and_then(|v| v.as_str()).unwrap_or_else(|| "buy".to_string());
            if direction != "buy" && direction != "sell" {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": format!("无法识别交易方向: {}", direction)}));
                continue;
            }

            let price = trade.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if price <= 0.0 {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "价格无效"}));
                continue;
            }

            let quantity = trade.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
            if quantity <= 0 {
                fail_count += 1;
                errors.push(value!({"index": idx as i64, "error": "数量无效"}));
                continue;
            }

            let name = trade.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let commission = trade.get("commission").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let remark = format!("截图识别导入");

            match trade_table.import_one_trade(
                account_id, &code, &name, &direction, price, quantity, commission, &remark,
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
            "rawContent": content,
        }))
    }
}
