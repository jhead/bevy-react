import { useState, type ReactNode } from 'react'
import {
  Node,
  Text,
  Button,
  TextInput,
  Checkbox,
  Slider,
  Select,
  ProgressBar,
  useInteraction,
} from 'bevy-react'
import type { BevyStyle } from 'bevy-react'

type SectionId =
  | 'buttons'
  | 'text'
  | 'inputs'
  | 'slider'
  | 'checkbox'
  | 'layout'

const SECTIONS: { id: SectionId; label: string }[] = [
  { id: 'buttons', label: 'Buttons' },
  { id: 'text', label: 'Text' },
  { id: 'inputs', label: 'Inputs' },
  { id: 'slider', label: 'Slider' },
  { id: 'checkbox', label: 'Checkbox' },
  { id: 'layout', label: 'Layout' },
]

function Section({
  title,
  children,
}: {
  title: string
  children: ReactNode
}): ReactNode {
  return (
    <Node
      style={{
        flexDirection: 'column',
        width: '100%',
        marginBottom: 28,
        padding: 16,
        backgroundColor: '#16213e',
        borderRadius: 10,
        borderWidth: 1,
        borderColor: '#0f3460',
      }}
    >
      <Text
        style={{
          fontSize: 18,
          color: '#ffffff',
          marginBottom: 12,
        }}
      >
        {title}
      </Text>
      {children}
    </Node>
  )
}

function GalleryButton({
  label,
  onClick,
  style,
}: {
  label: string
  onClick?: () => void
  style?: BevyStyle
}): ReactNode {
  const { hovered, pressed, handlers } = useInteraction()
  return (
    <Button
      {...handlers}
      onClick={onClick}
      style={{
        padding: 10,
        marginRight: 8,
        marginBottom: 8,
        borderRadius: 6,
        justifyContent: 'center',
        alignItems: 'center',
        backgroundColor: pressed
          ? '#c73a52'
          : hovered
            ? '#ff5a74'
            : '#e94560',
        ...style,
      }}
    >
      <Text style={{ fontSize: 14, color: '#ffffff' }}>{label}</Text>
    </Button>
  )
}

function NavChip({
  label,
  active,
  onClick,
}: {
  label: string
  active: boolean
  onClick: () => void
}): ReactNode {
  const { hovered, pressed, handlers } = useInteraction()
  return (
    <Button
      {...handlers}
      onClick={onClick}
      style={{
        padding: 8,
        marginRight: 8,
        marginBottom: 8,
        borderRadius: 6,
        justifyContent: 'center',
        alignItems: 'center',
        backgroundColor: active
          ? '#e94560'
          : pressed
            ? '#2a2a4a'
            : hovered
              ? '#3a3a5a'
              : '#1a1a2e',
        borderWidth: 1,
        borderColor: active ? '#ff8a9a' : '#3a3a5a',
      }}
    >
      <Text style={{ fontSize: 13, color: '#ffffff' }}>{label}</Text>
    </Button>
  )
}

function ButtonsSection(): ReactNode {
  const [clicks, setClicks] = useState(0)
  return (
    <Section title="Buttons">
      <Node style={{ flexDirection: 'row', flexWrap: 'wrap' }}>
        <GalleryButton
          label="Primary"
          onClick={() => setClicks((c) => c + 1)}
        />
        <GalleryButton
          label="Muted"
          onClick={() => setClicks((c) => c + 1)}
          style={{ backgroundColor: '#3a3a5a' }}
        />
        <GalleryButton
          label="Wide"
          onClick={() => setClicks((c) => c + 1)}
          style={{ minWidth: 160 }}
        />
      </Node>
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginTop: 4 }}>
        Clicks: {clicks}
      </Text>
    </Section>
  )
}

function TextSection(): ReactNode {
  return (
    <Section title="Text">
      <Text style={{ fontSize: 28, color: '#ffffff', marginBottom: 8 }}>
        Display heading
      </Text>
      <Text style={{ fontSize: 16, color: '#cccccc', marginBottom: 8 }}>
        Body copy for labels and descriptions.
      </Text>
      <Text style={{ fontSize: 12, color: '#8888aa' }}>
        Caption / helper text
      </Text>
    </Section>
  )
}

function InputsSection(): ReactNode {
  const [name, setName] = useState('')
  const [role, setRole] = useState('dev')
  return (
    <Section title="Inputs">
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginBottom: 6 }}>
        TextInput
      </Text>
      <TextInput
        value={name}
        onChange={setName}
        placeholder="Your name"
        style={{ width: '100%', marginBottom: 12 }}
      />
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginBottom: 6 }}>
        Select
      </Text>
      <Select
        value={role}
        onChange={setRole}
        options={[
          { value: 'dev', label: 'Developer' },
          { value: 'design', label: 'Designer' },
          { value: 'qa', label: 'QA' },
        ]}
        style={{ width: '100%' }}
      />
      <Text style={{ fontSize: 12, color: '#8888aa', marginTop: 8 }}>
        name={name || '(empty)'} · role={role}
      </Text>
    </Section>
  )
}

function SliderSection(): ReactNode {
  const [volume, setVolume] = useState(40)
  return (
    <Section title="Slider / Progress">
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginBottom: 6 }}>
        Volume ({volume})
      </Text>
      <Slider
        value={volume}
        onChange={setVolume}
        min={0}
        max={100}
        step={1}
        showValue
        style={{ width: '100%', marginBottom: 16 }}
      />
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginBottom: 6 }}>
        ProgressBar
      </Text>
      <ProgressBar value={volume} max={100} style={{ width: '100%' }} />
    </Section>
  )
}

function CheckboxSection(): ReactNode {
  const [a, setA] = useState(true)
  const [b, setB] = useState(false)
  return (
    <Section title="Checkbox">
      <Checkbox checked={a} onChange={setA} label="Enabled" style={{ marginBottom: 8 }} />
      <Checkbox checked={b} onChange={setB} label="Notifications" />
    </Section>
  )
}

function LayoutSection(): ReactNode {
  return (
    <Section title="Layout">
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginBottom: 8 }}>
        Row + gap
      </Text>
      <Node
        style={{
          flexDirection: 'row',
          gap: 8,
          marginBottom: 16,
          width: '100%',
        }}
      >
        {['A', 'B', 'C'].map((label) => (
          <Node
            key={label}
            style={{
              flexGrow: 1,
              height: 48,
              borderRadius: 6,
              backgroundColor: '#0f3460',
              justifyContent: 'center',
              alignItems: 'center',
            }}
          >
            <Text style={{ fontSize: 16, color: '#ffffff' }}>{label}</Text>
          </Node>
        ))}
      </Node>
      <Text style={{ fontSize: 13, color: '#aaaaaa', marginBottom: 8 }}>
        Column stack
      </Text>
      <Node style={{ flexDirection: 'column', gap: 6, width: '100%' }}>
        {[1, 2, 3].map((n) => (
          <Node
            key={n}
            style={{
              height: 36,
              borderRadius: 6,
              backgroundColor: '#1a1a2e',
              borderWidth: 1,
              borderColor: '#3a3a5a',
              justifyContent: 'center',
              paddingLeft: 12,
            }}
          >
            <Text style={{ fontSize: 14, color: '#cccccc' }}>Row {n}</Text>
          </Node>
        ))}
      </Node>
    </Section>
  )
}

function App(): ReactNode {
  const [section, setSection] = useState<SectionId | 'all'>('all')

  return (
    <Node
      style={{
        width: '100%',
        height: '100%',
        backgroundColor: '#0f0f1e',
        flexDirection: 'column',
        padding: 24,
        overflow: 'scroll',
      }}
    >
      <Text style={{ fontSize: 28, color: '#ffffff', marginBottom: 4 }}>
        Component Gallery
      </Text>
      <Text style={{ fontSize: 13, color: '#888888', marginBottom: 16 }}>
        Storybook-style samples — buttons, text, inputs, slider, checkbox, layout
      </Text>

      <Node style={{ flexDirection: 'row', flexWrap: 'wrap', marginBottom: 8 }}>
        <NavChip
          label="All"
          active={section === 'all'}
          onClick={() => setSection('all')}
        />
        {SECTIONS.map((s) => (
          <NavChip
            key={s.id}
            label={s.label}
            active={section === s.id}
            onClick={() => setSection(s.id)}
          />
        ))}
      </Node>

      {(section === 'all' || section === 'buttons') && <ButtonsSection />}
      {(section === 'all' || section === 'text') && <TextSection />}
      {(section === 'all' || section === 'inputs') && <InputsSection />}
      {(section === 'all' || section === 'slider') && <SliderSection />}
      {(section === 'all' || section === 'checkbox') && <CheckboxSection />}
      {(section === 'all' || section === 'layout') && <LayoutSection />}
    </Node>
  )
}

export default App
