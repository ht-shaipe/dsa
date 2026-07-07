import { defineStore } from 'pinia'

export const useAnalysisStore = defineStore('analysis', {
  state: () => ({
    currentReport: null as any,
    isAnalyzing: false,
    recentReports: [] as any[],
  }),
  actions: {
    setReport(report: any) {
      this.currentReport = report
    },
    setAnalyzing(v: boolean) {
      this.isAnalyzing = v
    },
    setRecentReports(reports: any[]) {
      this.recentReports = reports
    },
    clearReport() {
      this.currentReport = null
    },
  },
})
