import { defineStore } from 'pinia'

export const useAnalysisStore = defineStore('analysis', {
  state: () => ({
    currentReport: null as any,
    isAnalyzing: false,
    streamingText: '',
    streamStatus: '',
    recentReports: [] as any[],
  }),
  actions: {
    setReport(report: any) {
      this.currentReport = report
    },
    setAnalyzing(v: boolean) {
      this.isAnalyzing = v
      if (v) {
        this.streamingText = ''
        this.streamStatus = ''
      }
    },
    appendStreamText(chunk: string) {
      this.streamingText += chunk
    },
    setStreamStatus(status: string) {
      this.streamStatus = status
    },
    clearStreamState() {
      this.streamingText = ''
      this.streamStatus = ''
    },
    setRecentReports(reports: any[]) {
      this.recentReports = reports
    },
    clearReport() {
      this.currentReport = null
      this.streamingText = ''
      this.streamStatus = ''
    },
  },
})
