import { callApi } from './index'

export const indicatorApi = {
  status: () => callApi('indicator', 'status'),
  calcAll: () => callApi('indicator', 'calc_all'),
  calcStock: (code: string) => callApi('indicator', 'calc_stock', { code }),
}
