//! 报告渲染 - 将 AnalysisReport 渲染为文本/Markdown

use dsa_core::models::AnalysisReport;

pub struct ReportRenderer;

impl ReportRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_markdown(&self, report: &AnalysisReport) -> String {
        let mut md = String::new();

        // 标题
        if let Some(ref name) = report.stock_name {
            md.push_str(&format!("# {} 分析报告\n\n", name));
        }

        // 核心决策
        if let Some(ref _decision) = report.decision_type {
            md.push_str(&format!(
                "**决策**: {} {}  ",
                report.action_emoji(),
                report.action_label()
            ));
        }
        if let Some(score) = report.sentiment_score {
            md.push_str(&format!(
                "**评分**: {} {}分  ",
                report.score_emoji(),
                score
            ));
        }
        if let Some(ref confidence) = report.confidence_level {
            md.push_str(&format!("**信心**: {}", confidence));
        }
        md.push_str("\n\n");

        // 综合摘要
        if let Some(ref summary) = report.analysis_summary {
            md.push_str(&format!("## 综合摘要\n{}\n\n", summary));
        }

        // Dashboard 各子模块
        if let Some(ref dashboard) = report.dashboard {
            // 核心结论
            if let Some(ref cc) = dashboard.core_conclusion {
                md.push_str("## 核心结论\n");
                if let Some(ref s) = cc.one_sentence {
                    md.push_str(&format!("- 一句话结论: {}\n", s));
                }
                if let Some(ref st) = cc.signal_type {
                    md.push_str(&format!("- 信号类型: {}\n", st));
                }
                if let Some(ref ts) = cc.time_sensitivity {
                    md.push_str(&format!("- 时间敏感度: {}\n", ts));
                }
                if let Some(ref pa) = cc.position_advice {
                    if let Some(ref np) = pa.no_position {
                        md.push_str(&format!("- 无仓位建议: {}\n", np));
                    }
                    if let Some(ref hp) = pa.has_position {
                        md.push_str(&format!("- 有仓位建议: {}\n", hp));
                    }
                }
                md.push('\n');
            }

            // 数据视角
            if let Some(ref dp) = dashboard.data_perspective {
                md.push_str("## 数据视角\n");

                if let Some(ref ts) = dp.trend_status {
                    md.push_str("### 趋势状态\n");
                    if let Some(ref ma) = ts.ma_alignment {
                        md.push_str(&format!("- 均线排列: {}\n", ma));
                    }
                    if let Some(bullish) = ts.is_bullish {
                        md.push_str(&format!(
                            "- 多头趋势: {}\n",
                            if bullish { "是" } else { "否" }
                        ));
                    }
                    if let Some(score) = ts.trend_score {
                        md.push_str(&format!("- 趋势评分: {:.0}\n", score));
                    }
                }

                if let Some(ref pp) = dp.price_position {
                    md.push_str("### 价格位置\n");
                    if let Some(p) = pp.current_price {
                        md.push_str(&format!("- 当前价: {:.2}\n", p));
                    }
                    if let Some(m) = pp.ma5 {
                        md.push_str(&format!("- MA5: {:.2}\n", m));
                    }
                    if let Some(m) = pp.ma10 {
                        md.push_str(&format!("- MA10: {:.2}\n", m));
                    }
                    if let Some(m) = pp.ma20 {
                        md.push_str(&format!("- MA20: {:.2}\n", m));
                    }
                    if let Some(b) = pp.bias_ma5 {
                        md.push_str(&format!("- 乖离率MA5: {:.2}%\n", b));
                    }
                    if let Some(ref s) = pp.bias_status {
                        md.push_str(&format!("- 偏离状态: {}\n", s));
                    }
                    if let Some(s) = pp.support_level {
                        md.push_str(&format!("- 支撑位: {:.2}\n", s));
                    }
                    if let Some(r) = pp.resistance_level {
                        md.push_str(&format!("- 阻力位: {:.2}\n", r));
                    }
                }

                if let Some(ref va) = dp.volume_analysis {
                    md.push_str("### 量能分析\n");
                    if let Some(vr) = va.volume_ratio {
                        md.push_str(&format!("- 量比: {:.2}\n", vr));
                    }
                    if let Some(ref s) = va.volume_status {
                        md.push_str(&format!("- 量能状态: {}\n", s));
                    }
                    if let Some(tr) = va.turnover_rate {
                        md.push_str(&format!("- 换手率: {:.2}%\n", tr));
                    }
                    if let Some(ref m) = va.volume_meaning {
                        md.push_str(&format!("- 量能含义: {}\n", m));
                    }
                }

                if let Some(ref cs) = dp.chip_structure {
                    md.push_str("### 筹码结构\n");
                    if let Some(pr) = cs.profit_ratio {
                        md.push_str(&format!("- 获利比例: {:.1}%\n", pr));
                    }
                    if let Some(ac) = cs.avg_cost {
                        md.push_str(&format!("- 平均成本: {:.2}\n", ac));
                    }
                    if let Some(c) = cs.concentration {
                        md.push_str(&format!("- 集中度: {:.1}\n", c));
                    }
                    if let Some(ref h) = cs.chip_health {
                        md.push_str(&format!("- 筹码健康度: {}\n", h));
                    }
                }

                md.push('\n');
            }

            // 情报面
            if let Some(ref intel) = dashboard.intelligence {
                md.push_str("## 情报面\n");
                if let Some(ref n) = intel.latest_news {
                    md.push_str(&format!("- 最新消息: {}\n", n));
                }
                if let Some(ref alerts) = intel.risk_alerts {
                    if !alerts.is_empty() {
                        md.push_str(&format!("- 风险提示: {}\n", alerts.join("; ")));
                    }
                }
                if let Some(ref catalysts) = intel.positive_catalysts {
                    if !catalysts.is_empty() {
                        md.push_str(&format!("- 利好催化: {}\n", catalysts.join("; ")));
                    }
                }
                if let Some(ref eo) = intel.earnings_outlook {
                    md.push_str(&format!("- 业绩预期: {}\n", eo));
                }
                if let Some(ref ss) = intel.sentiment_summary {
                    md.push_str(&format!("- 舆情总结: {}\n", ss));
                }
                md.push('\n');
            }

            // 作战计划
            if let Some(ref bp) = dashboard.battle_plan {
                md.push_str("## 作战计划\n");

                if let Some(ref sp) = bp.sniper_points {
                    md.push_str("### 狙击点位\n");
                    if let Some(p) = sp.ideal_buy {
                        md.push_str(&format!("- 理想买入: {:.2}\n", p));
                    }
                    if let Some(p) = sp.secondary_buy {
                        md.push_str(&format!("- 次级买入: {:.2}\n", p));
                    }
                    if let Some(p) = sp.stop_loss {
                        md.push_str(&format!("- 止损: {:.2}\n", p));
                    }
                    if let Some(p) = sp.take_profit {
                        md.push_str(&format!("- 止盈: {:.2}\n", p));
                    }
                }

                if let Some(ref ps) = bp.position_strategy {
                    md.push_str("### 仓位策略\n");
                    if let Some(ref sp) = ps.suggested_position {
                        md.push_str(&format!("- 建议仓位: {}\n", sp));
                    }
                    if let Some(ref ep) = ps.entry_plan {
                        md.push_str(&format!("- 建仓计划: {}\n", ep));
                    }
                    if let Some(ref rc) = ps.risk_control {
                        md.push_str(&format!("- 风控措施: {}\n", rc));
                    }
                }

                if let Some(ref cl) = bp.action_checklist {
                    if !cl.is_empty() {
                        md.push_str("### 操作检查清单\n");
                        for item in cl {
                            md.push_str(&format!("- [ ] {}\n", item));
                        }
                    }
                }

                md.push('\n');
            }

            // 阶段决策
            if let Some(ref pd) = dashboard.phase_decision {
                md.push_str("## 阶段决策\n");
                if let Some(ref aw) = pd.action_window {
                    md.push_str(&format!("- 操作窗口: {}\n", aw));
                }
                if let Some(ref ia) = pd.immediate_action {
                    md.push_str(&format!("- 即时操作: {}\n", ia));
                }
                if !pd.watch_conditions.is_empty() {
                    md.push_str(&format!("- 观察条件: {}\n", pd.watch_conditions.join("; ")));
                }
                if !pd.data_limitations.is_empty() {
                    md.push_str(&format!(
                        "- 数据限制: {}\n",
                        pd.data_limitations.join("; ")
                    ));
                }
                md.push('\n');
            }

            // 信号归因
            if let Some(ref sa) = dashboard.signal_attribution {
                md.push_str("## 信号归因\n");
                if let Some(t) = sa.technical_indicators {
                    md.push_str(&format!("- 技术指标: {:.0}%\n", t));
                }
                if let Some(n) = sa.news_sentiment {
                    md.push_str(&format!("- 消息面: {:.0}%\n", n));
                }
                if let Some(f) = sa.fundamentals {
                    md.push_str(&format!("- 基本面: {:.0}%\n", f));
                }
                if let Some(m) = sa.market_conditions {
                    md.push_str(&format!("- 大盘环境: {:.0}%\n", m));
                }
                if let Some(ref s) = sa.strongest_bullish_signal {
                    md.push_str(&format!("- 最强看多信号: {}\n", s));
                }
                if let Some(ref s) = sa.strongest_bearish_signal {
                    md.push_str(&format!("- 最强看空信号: {}\n", s));
                }
                md.push('\n');
            }
        }

        // 风险提示
        if let Some(ref risk) = report.risk_warning {
            md.push_str(&format!("## ⚠️ 风险提示\n{}\n\n", risk));
        }

        // 趋势分析
        if let Some(ref ta) = report.trend_analysis {
            md.push_str(&format!("## 趋势分析\n{}\n\n", ta));
        }

        // 短期展望
        if let Some(ref st) = report.short_term_outlook {
            md.push_str(&format!("## 短期展望\n{}\n\n", st));
        }

        // 中期展望
        if let Some(ref mt) = report.medium_term_outlook {
            md.push_str(&format!("## 中期展望\n{}\n\n", mt));
        }

        // 技术分析
        if let Some(ref ta) = report.technical_analysis {
            md.push_str(&format!("## 技术分析\n{}\n\n", ta));
        }

        // 均线分析
        if let Some(ref ma) = report.ma_analysis {
            md.push_str(&format!("## 均线分析\n{}\n\n", ma));
        }

        // 量价分析
        if let Some(ref va) = report.volume_analysis {
            md.push_str(&format!("## 量价分析\n{}\n\n", va));
        }

        // 基本面分析
        if let Some(ref fa) = report.fundamental_analysis {
            md.push_str(&format!("## 基本面分析\n{}\n\n", fa));
        }

        // 市场情绪
        if let Some(ref ms) = report.market_sentiment {
            md.push_str(&format!("## 市场情绪\n{}\n\n", ms));
        }

        md
    }

    pub fn render_text(&self, report: &AnalysisReport) -> String {
        let mut text = String::new();

        // 基本信息
        if let Some(ref name) = report.stock_name {
            text.push_str(&format!("【{}】", name));
        }
        if let Some(ref _decision) = report.decision_type {
            text.push_str(&format!(
                " {}{}",
                report.action_emoji(),
                report.action_label()
            ));
        }
        if let Some(score) = report.sentiment_score {
            text.push_str(&format!(" 评分:{}", score));
        }
        if let Some(ref confidence) = report.confidence_level {
            text.push_str(&format!(" 信心:{}", confidence));
        }
        if let Some(ref advice) = report.operation_advice {
            text.push_str(&format!(" | {}", advice));
        }

        // 综合摘要
        if let Some(ref summary) = report.analysis_summary {
            text.push_str(&format!("\n\n摘要: {}", summary));
        }

        // 关键要点
        if let Some(ref kp) = report.key_points {
            text.push_str(&format!("\n要点: {}", kp));
        }

        // 风险提示
        if let Some(ref risk) = report.risk_warning {
            text.push_str(&format!("\n⚠️风险: {}", risk));
        }

        // Dashboard 核心结论
        if let Some(ref dashboard) = report.dashboard {
            if let Some(ref cc) = dashboard.core_conclusion {
                if let Some(ref s) = cc.one_sentence {
                    text.push_str(&format!("\n结论: {}", s));
                }
            }

            // 作战计划要点
            if let Some(ref bp) = dashboard.battle_plan {
                if let Some(ref sp) = bp.sniper_points {
                    let mut points = Vec::new();
                    if let Some(p) = sp.ideal_buy {
                        points.push(format!("买入{:.2}", p));
                    }
                    if let Some(p) = sp.stop_loss {
                        points.push(format!("止损{:.2}", p));
                    }
                    if let Some(p) = sp.take_profit {
                        points.push(format!("止盈{:.2}", p));
                    }
                    if !points.is_empty() {
                        text.push_str(&format!("\n计划: {}", points.join(" ")));
                    }
                }
            }
        }

        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::models::AnalysisReport;

    fn make_report() -> AnalysisReport {
        AnalysisReport {
            stock_name: Some("贵州茅台".to_string()),
            sentiment_score: Some(8),
            decision_type: Some("buy".to_string()),
            operation_advice: Some("建议买入".to_string()),
            confidence_level: Some("high".to_string()),
            analysis_summary: Some("技术面看多".to_string()),
            risk_warning: Some("注意回调风险".to_string()),
            trend_prediction: None,
            dashboard: None,
            key_points: None,
            buy_reason: None,
            trend_analysis: None,
            short_term_outlook: None,
            medium_term_outlook: None,
            technical_analysis: None,
            ma_analysis: None,
            volume_analysis: None,
            pattern_analysis: None,
            fundamental_analysis: None,
            sector_position: None,
            company_highlights: None,
            news_summary: None,
            market_sentiment: None,
            hot_topics: None,
            search_performed: None,
            data_sources: None,
        }
    }

    #[test]
    fn test_render_markdown_basic() {
        let report = make_report();
        let renderer = ReportRenderer::new();
        let md = renderer.render_markdown(&report);
        assert!(md.contains("贵州茅台"));
        assert!(md.contains("买入"), "should contain action label '买入' from decision_type");
        assert!(md.contains("注意回调风险"));
    }

    #[test]
    fn test_render_text_basic() {
        let report = make_report();
        let renderer = ReportRenderer::new();
        let text = renderer.render_text(&report);
        assert!(text.contains("贵州茅台"));
        assert!(text.contains("8"));
        assert!(text.contains("建议买入"));
    }

    #[test]
    fn test_render_empty_report() {
        let report = AnalysisReport {
            stock_name: None,
            sentiment_score: None,
            decision_type: None,
            operation_advice: None,
            confidence_level: None,
            analysis_summary: None,
            risk_warning: None,
            trend_prediction: None,
            dashboard: None,
            key_points: None,
            buy_reason: None,
            trend_analysis: None,
            short_term_outlook: None,
            medium_term_outlook: None,
            technical_analysis: None,
            ma_analysis: None,
            volume_analysis: None,
            pattern_analysis: None,
            fundamental_analysis: None,
            sector_position: None,
            company_highlights: None,
            news_summary: None,
            market_sentiment: None,
            hot_topics: None,
            search_performed: None,
            data_sources: None,
        };
        let renderer = ReportRenderer::new();
        let md = renderer.render_markdown(&report);
        let _text = renderer.render_text(&report);
        // Empty report with all None fields produces minimal output
        assert!(!md.is_empty(), "markdown should produce at least some structure");
        // text may be empty when all fields are None, that's fine
    }
}
