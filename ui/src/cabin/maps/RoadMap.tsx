import { useEffect, useLayoutEffect, useRef } from 'react'
import { useLook } from '../state/hooks'
import { setLiveMapAppearance, showLiveMap } from './liveMap'
import type { MapData } from './mapData'
import { isInHost } from '../../shared/host'
import { setHaulMpEnabled, useHaulMp } from './haulmp'
import './maps.css'

/**
 * The live map: the game's own roads and land, drawn in WestPlay's style and following the truck (or
 * resting on the exported area when there's no truck, e.g. in a browser). One map serves every screen
 * (liveMap.ts); this is where it's shown. `interactive`: the player can move it (the Maps app).
 */
export function RoadMap({ data, interactive }: { data: MapData; interactive: boolean }) {
  const host = useRef<HTMLDivElement>(null)
  const haulMpEnabled = useHaulMp((state) => state.enabled)
  const { appearance } = useLook()
  // The appearance as of now, for when the map is shown (outside render).
  const appearanceRef = useRef(appearance)
  useLayoutEffect(() => {
    appearanceRef.current = appearance
  })

  useEffect(() => {
    if (!host.current) return
    return showLiveMap(host.current, data, appearanceRef.current, interactive)
  }, [data, interactive])

  // Light and dark: restyled in place, fading like the rest of the UI.
  useEffect(() => setLiveMapAppearance(appearance), [appearance])

  return (
    <div className="road-map-surface">
      <div ref={host} className="road-map-host" />
      {isInHost() && (
        <button
          className={`haulmp-toggle${haulMpEnabled ? ' active' : ''}`}
          type="button"
          aria-pressed={haulMpEnabled}
          aria-label={haulMpEnabled ? 'Hide HaulMP players' : 'Show HaulMP players'}
          title="HaulMP players"
          onClick={(event) => {
            event.stopPropagation()
            setHaulMpEnabled(!haulMpEnabled)
          }}
        >
          HaulMP
        </button>
      )}
    </div>
  )
}
