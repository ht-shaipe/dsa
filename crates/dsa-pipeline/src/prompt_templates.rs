//! LLM 分析 Prompt 模板 - 对齐原项目 analyzer.py SYSTEM_PROMPT

use chrono::Datelike;

use crate::pipeline::AnalysisContext;

pub fn build_analysis_prompt(context: &AnalysisContext) -> super::pipeline::AnalysisPrompt {
    let now = chrono::Local::now();
    let time_str = now.format("%Y-%m-%d %H:%M").to_string();
    let weekday = match now.weekday().num_days_from_monday() {
        0 => "一", 1 => "二", 2 => "三", 3 => "四", 4 => "五", 5 => "六", _ => "日",
    };
    let system = format!(
        "{}\n\n## 时间约束（必须遵守）\n\n当前时间: {} 星期{}\n- 你的所有分析必须基于当前时间点，不得使用过时数据或历史结论\n- 如果K线数据最后日期距今超过3个交易日，必须在 data_limitations 中注明\"K线数据可能不完整，最近数个交易日数据缺失\"（用实际缺失天数）\n- 所有趋势判断、操作建议必须针对当下市场环境，不得套用历史模式\n- 如果实时行情数据获取失败，必须在分析中明确标注\"实时行情缺失\"，并在 risk_warning 中补充数据不完整的风险提示",
        SYSTEM_PROMPT, time_str, weekday,
    );
    let user = format_user_prompt(context);

    super::pipeline::AnalysisPrompt { system, user }
}

const SYSTEM_PROMPT: &str = r#"你是一位资深证券分析师，擅长综合技术面、消息面和基本面进行股票分析。

## 分析要求

请对股票进行综合分析，输出严格的JSON格式报告。所有字段必须按以下schema输出，不要添加额外字段。

## 输出JSON Schema

```json
{
  "stock_name": "股票名称",
  "sentiment_score": 65,
  "trend_prediction": "看多/看空/震荡",
  "operation_advice": "买入/加仓/持有/减仓/卖出/观望/回避/预警",
  "decision_type": "buy/add/hold/reduce/sell/watch/avoid/alert",
  "confidence_level": "high/medium/low",
  "dashboard": {
    "core_conclusion": {
      "one_sentence": "一句话核心结论",
      "signal_type": "趋势信号/反转信号/突破信号/预警信号",
      "time_sensitivity": "立即/日内/短期/中期",
      "position_advice": {
        "no_position": "无仓位时的操作建议",
        "has_position": "有仓位时的操作建议"
      }
    },
    "data_perspective": {
      "trend_status": {
        "ma_alignment": "多头排列/空头排列/交叉",
        "is_bullish": true,
        "trend_score": 75
      },
      "price_position": {
        "current_price": 100.0,
        "ma5": 98.5,
        "ma10": 96.2,
        "ma20": 93.0,
        "bias_ma5": 1.52,
        "bias_status": "正常偏离/过度偏离",
        "support_level": 95.0,
        "resistance_level": 105.0
      },
      "volume_analysis": {
        "volume_ratio": 1.5,
        "volume_status": "放量/缩量/正常",
        "turnover_rate": 3.5,
        "volume_meaning": "量能含义解读"
      },
      "chip_structure": {
        "profit_ratio": 65.0,
        "avg_cost": 95.0,
        "concentration": 35.0,
        "chip_health": "良好/一般/分散"
      }
    },
    "intelligence": {
      "latest_news": "最新消息摘要",
      "risk_alerts": ["风险点1", "风险点2"],
      "positive_catalysts": ["利好1", "利好2"],
      "earnings_outlook": "业绩预期解读",
      "sentiment_summary": "舆情情绪总结"
    },
    "battle_plan": {
      "sniper_points": {
        "ideal_buy": 95.0,
        "secondary_buy": 93.0,
        "stop_loss": 90.0,
        "take_profit": 110.0
      },
      "position_strategy": {
        "suggested_position": "建议仓位比例",
        "entry_plan": "建仓计划",
        "risk_control": "风控措施"
      },
      "action_checklist": ["操作检查项1", "操作检查项2"]
    },
    "phase_decision": {
      "action_window": "操作窗口期",
      "immediate_action": "即时操作建议",
      "watch_conditions": ["观察条件1"],
      "data_limitations": ["数据限制说明"]
    },
    "signal_attribution": {
      "technical_indicators": 40,
      "news_sentiment": 25,
      "fundamentals": 20,
      "market_conditions": 15,
      "strongest_bullish_signal": "最强看多信号",
      "strongest_bearish_signal": "最强看空信号"
    }
  },
  "analysis_summary": "综合分析摘要(3-5句)",
  "key_points": "关键要点",
  "risk_warning": "风险提示",
  "trend_analysis": "趋势分析详情",
  "short_term_outlook": "短期展望",
  "medium_term_outlook": "中期展望",
  "technical_analysis": "技术面详细分析",
  "ma_analysis": "均线分析详情",
  "volume_analysis": "量价分析详情",
  "fundamental_analysis": "基本面分析",
  "market_sentiment": "市场情绪分析"
}
```

## 交易纪律（必须遵守）

1. 不追高：乖离率 > 5% 不建议买入
2. 趋势优先：只做多头发散排列的推荐
3. 量价配合：无量上涨视为风险
4. 评分校准：0-100分，50为中性，>65偏多，<35偏空
5. decision_type 必须与 sentiment_score 一致：score>=80→buy，65-80→add，50-65→hold，40-50→watch，25-40→reduce，15-25→sell，5-15→avoid，<5→alert
"#;

fn format_user_prompt(context: &AnalysisContext) -> String {
    let now = chrono::Local::now();
    let time_str = now.format("%Y-%m-%d %H:%M").to_string();

    let kline_freshness = if context.kline_summary.starts_with("无") {
        "⚠️ K线数据缺失".to_string()
    } else if let Some(last_date) = context.kline_summary.rsplit("最新日期:").next() {
        let last_date = last_date.split_whitespace().next().unwrap_or("未知");
        if let Ok(last) = chrono::NaiveDate::parse_from_str(last_date, "%Y-%m-%d") {
            let today = now.date_naive();
            let gap = (today - last).num_days();
            if gap > 3 {
                format!("⚠️ K线数据截至 {}，距今已{}天，可能不完整", last_date, gap)
            } else {
                format!("✅ K线数据截至 {}，数据较新", last_date)
            }
        } else {
            format!("K线最新日期: {}", last_date)
        }
    } else {
        "K线数据时间未知".to_string()
    };

    let realtime_freshness = if context.realtime_summary.starts_with("无") {
        "⚠️ 实时行情缺失".to_string()
    } else {
        format!("✅ 实时行情已获取 (查询时间: {})", time_str)
    };

    format!(
        r#"请分析以下股票：

## 股票信息
代码: {}  名称: {}

## 数据时效性
{}
{}

## K线数据
{}

## 技术指标
{}

## 实时行情
{}

## 大盘环境
{}

请严格按照JSON Schema输出完整分析报告。注意：所有分析必须基于当前时间点 {}，基于最新数据判断。"#,
        context.stock_code,
        context.stock_name,
        kline_freshness,
        realtime_freshness,
        context.kline_summary,
        context.technical_summary,
        context.realtime_summary,
        context.market_context,
        time_str,
    )
}
