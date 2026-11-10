import { create } from 'zustand'
import { postToHost } from '../../shared/host'

interface HaulMpState {
  enabled: boolean
}

/** The optional live-map layer is off until the player turns it on. */
export const useHaulMp = create<HaulMpState>(() => ({ enabled: false }))

export function setHaulMpEnabled(on: boolean) {
  useHaulMp.setState({ enabled: on })
  postToHost({ type: 'haulMp', on })
}
