import { useState, type ReactNode } from 'react'
import { Node, Text, Button, useInteraction } from 'bevy-react'
import type { BevyStyle } from 'bevy-react'

type Screen = 'main' | 'play' | 'options' | 'credits'

function MenuButton({
  label,
  onClick,
  style,
}: {
  label: string
  onClick: () => void
  style?: BevyStyle
}): ReactNode {
  const { hovered, pressed, handlers } = useInteraction()

  return (
    <Button
      {...handlers}
      onClick={onClick}
      style={{
        width: '100%',
        maxWidth: 320,
        padding: 14,
        marginBottom: 12,
        borderRadius: 8,
        justifyContent: 'center',
        alignItems: 'center',
        backgroundColor: pressed
          ? '#3a3a6a'
          : hovered
            ? '#4a4a7a'
            : '#2a2a4a',
        borderWidth: 2,
        borderColor: hovered ? '#8a8aff' : '#4a4a6a',
        ...style,
      }}
    >
      <Text style={{ fontSize: 20, color: '#ffffff' }}>{label}</Text>
    </Button>
  )
}

function MainMenu({ go }: { go: (s: Screen) => void }): ReactNode {
  return (
    <Node
      style={{
        flexDirection: 'column',
        alignItems: 'center',
        width: '100%',
        maxWidth: 400,
      }}
    >
      <Text style={{ fontSize: 42, color: '#ffffff', marginBottom: 8 }}>
        Bevy React
      </Text>
      <Text
        style={{ fontSize: 16, color: '#8888aa', marginBottom: 32 }}
      >
        Main Menu
      </Text>
      <MenuButton label="Play" onClick={() => go('play')} />
      <MenuButton label="Options" onClick={() => go('options')} />
      <MenuButton label="Credits" onClick={() => go('credits')} />
    </Node>
  )
}

function Panel({
  title,
  body,
  onBack,
}: {
  title: string
  body: string
  onBack: () => void
}): ReactNode {
  return (
    <Node
      style={{
        flexDirection: 'column',
        alignItems: 'center',
        width: '100%',
        maxWidth: 420,
        padding: 24,
        backgroundColor: '#16213e',
        borderRadius: 12,
        borderWidth: 2,
        borderColor: '#0f3460',
      }}
    >
      <Text style={{ fontSize: 28, color: '#e94560', marginBottom: 16 }}>
        {title}
      </Text>
      <Text
        style={{
          fontSize: 16,
          color: '#ccccdd',
          marginBottom: 24,
          textAlign: 'center',
        }}
      >
        {body}
      </Text>
      <MenuButton label="Back" onClick={onBack} />
    </Node>
  )
}

function App(): ReactNode {
  const [screen, setScreen] = useState<Screen>('main')

  return (
    <Node
      style={{
        width: '100%',
        height: '100%',
        backgroundColor: '#0a0a14',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 32,
      }}
    >
      {screen === 'main' && <MainMenu go={setScreen} />}
      {screen === 'play' && (
        <Panel
          title="Play"
          body="Start a match from here once game-state binding lands."
          onBack={() => setScreen('main')}
        />
      )}
      {screen === 'options' && (
        <Panel
          title="Options"
          body="Audio and graphics would go here. See the forms example for inputs."
          onBack={() => setScreen('main')}
        />
      )}
      {screen === 'credits' && (
        <Panel
          title="Credits"
          body="bevy-react — React UI renderer for Bevy."
          onBack={() => setScreen('main')}
        />
      )}
    </Node>
  )
}

export default App
