import { callApi } from './index'

export const authApi = {
  settings: () => callApi('system', 'settings'),
}
