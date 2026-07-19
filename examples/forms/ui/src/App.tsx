import { useState, type ReactNode } from 'react'
import {
  Node,
  Text,
  Button,
  TextInput,
  Checkbox,
  Slider,
  Select,
} from 'bevy-react'

type Quality = 'low' | 'medium' | 'high'

function Field({
  label,
  children,
}: {
  label: string
  children: ReactNode
}): ReactNode {
  return (
    <Node
      style={{
        flexDirection: 'column',
        width: '100%',
        marginBottom: 20,
      }}
    >
      <Text
        style={{
          fontSize: 14,
          color: '#aaaaaa',
          marginBottom: 8,
        }}
      >
        {label}
      </Text>
      {children}
    </Node>
  )
}

function App(): ReactNode {
  const [displayName, setDisplayName] = useState('')
  const [email, setEmail] = useState('')
  const [music, setMusic] = useState(true)
  const [sfx, setSfx] = useState(true)
  const [volume, setVolume] = useState(70)
  const [quality, setQuality] = useState<Quality>('medium')
  const [saved, setSaved] = useState(false)

  const nameOk = displayName.trim().length >= 2
  const emailOk = email.includes('@') && email.includes('.')
  const canSave = nameOk && emailOk

  return (
    <Node
      style={{
        width: '100%',
        height: '100%',
        backgroundColor: '#0f0f1e',
        flexDirection: 'column',
        alignItems: 'center',
        padding: 32,
        overflow: 'scroll',
      }}
    >
      <Text
        style={{
          fontSize: 32,
          color: '#ffffff',
          marginBottom: 8,
        }}
      >
        Settings
      </Text>
      <Text
        style={{
          fontSize: 14,
          color: '#888888',
          marginBottom: 24,
        }}
      >
        Forms example — TextInput, Checkbox, Slider, Select
      </Text>

      <Node
        style={{
          width: '100%',
          maxWidth: 440,
          padding: 24,
          backgroundColor: '#16213e',
          borderRadius: 12,
          borderWidth: 2,
          borderColor: '#0f3460',
          flexDirection: 'column',
        }}
      >
        <Field label="Display name">
          <TextInput
            value={displayName}
            onChange={(v) => {
              setDisplayName(v)
              setSaved(false)
            }}
            placeholder="At least 2 characters"
            style={{ width: '100%' }}
          />
          {!nameOk && displayName.length > 0 ? (
            <Text style={{ fontSize: 12, color: '#e94560', marginTop: 6 }}>
              Name must be at least 2 characters
            </Text>
          ) : null}
        </Field>

        <Field label="Email">
          <TextInput
            value={email}
            onChange={(v) => {
              setEmail(v)
              setSaved(false)
            }}
            placeholder="you@example.com"
            style={{ width: '100%' }}
          />
          {!emailOk && email.length > 0 ? (
            <Text style={{ fontSize: 12, color: '#e94560', marginTop: 6 }}>
              Enter a valid email
            </Text>
          ) : null}
        </Field>

        <Field label="Audio">
          <Checkbox
            checked={music}
            onChange={setMusic}
            label="Music"
            style={{ marginBottom: 8 }}
          />
          <Checkbox checked={sfx} onChange={setSfx} label="Sound effects" />
        </Field>

        <Field label={`Master volume (${volume})`}>
          <Slider
            value={volume}
            onChange={setVolume}
            min={0}
            max={100}
            step={5}
            showValue
            style={{ width: '100%' }}
          />
        </Field>

        <Field label="Graphics quality">
          <Select
            value={quality}
            onChange={setQuality}
            options={[
              { value: 'low', label: 'Low' },
              { value: 'medium', label: 'Medium' },
              { value: 'high', label: 'High' },
            ]}
            style={{ width: '100%' }}
          />
        </Field>

        <Button
          onClick={() => {
            if (canSave) setSaved(true)
          }}
          style={{
            padding: 12,
            borderRadius: 8,
            justifyContent: 'center',
            alignItems: 'center',
            backgroundColor: canSave ? '#e94560' : '#3a3a4a',
            marginTop: 8,
          }}
        >
          <Text style={{ fontSize: 16, color: '#ffffff' }}>
            {saved ? 'Saved' : 'Save settings'}
          </Text>
        </Button>
      </Node>
    </Node>
  )
}

export default App
