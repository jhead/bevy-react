import type { ReactNode } from 'react'
import {
  Node,
  Text,
  Button,
  ProgressBar,
  useBridgeState,
  callNative,
  useInteraction,
} from 'bevy-react'

type Hud = {
  hp: number
  max_hp: number
  score: number
  hp_ratio: number
}

const INITIAL: Hud = { hp: 100, max_hp: 100, score: 0, hp_ratio: 1 }

function ActionButton({
  label,
  onClick,
}: {
  label: string
  onClick: () => void
}): ReactNode {
  const { hovered, pressed, handlers } = useInteraction()

  return (
    <Button
      {...handlers}
      onClick={onClick}
      style={{
        padding: 10,
        marginRight: 10,
        borderRadius: 6,
        justifyContent: 'center',
        alignItems: 'center',
        backgroundColor: pressed
          ? '#1a5c3a'
          : hovered
            ? '#247a4c'
            : '#163d2a',
        borderWidth: 1,
        borderColor: hovered ? '#4dff9a' : '#2a6a48',
      }}
    >
      <Text style={{ fontSize: 14, color: '#e8ffe8' }}>{label}</Text>
    </Button>
  )
}

function App(): ReactNode {
  const hud = useBridgeState<Hud>('hud', INITIAL)

  return (
    <Node
      style={{
        width: '100%',
        height: '100%',
        // Transparent playfield; HUD chrome only in the corners.
        backgroundColor: 'rgba(8, 12, 18, 0.35)',
        padding: 20,
        flexDirection: 'column',
        justifyContent: 'spaceBetween',
      }}
    >
      <Node
        style={{
          flexDirection: 'row',
          justifyContent: 'spaceBetween',
          alignItems: 'start',
          width: '100%',
        }}
      >
        <Node style={{ flexDirection: 'column', width: 280 }}>
          <Text
            style={{
              fontSize: 14,
              color: '#9ab0c0',
              marginBottom: 6,
              textShadow: '1px 1px 0 #000',
            }}
          >
            {`HP ${hud.hp} / ${hud.max_hp}`}
          </Text>
          <ProgressBar
            progress={hud.hp_ratio}
            style={{ width: '100%', height: 18 }}
            trackStyle={{
              backgroundColor: '#1a222c',
              borderRadius: 4,
              borderWidth: 1,
              borderColor: '#334455',
            }}
            fillStyle={{
              backgroundColor: hud.hp_ratio > 0.3 ? '#3ecf7a' : '#e94560',
              borderRadius: 3,
            }}
          />
        </Node>

        <Text
          style={{
            fontSize: 22,
            color: '#ffe566',
            textShadow: '2px 2px 0 #000',
          }}
        >
          {`Score ${hud.score}`}
        </Text>
      </Node>

      <Node style={{ flexDirection: 'row', alignItems: 'center' }}>
        <ActionButton
          label="+10 Score"
          onClick={() => callNative('add_score', 10)}
        />
        <ActionButton label="Heal" onClick={() => callNative('heal')} />
        <Text style={{ fontSize: 12, color: '#8899aa', marginLeft: 8 }}>
          ECS → ReactBridge → useBridgeState
        </Text>
      </Node>
    </Node>
  )
}

export default App
