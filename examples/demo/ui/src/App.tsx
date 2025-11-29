import { useState } from 'react'
import { Node, Text, Button, TextInput } from 'bevy-react'

// Header component
function Header() {
  return (
    <Node
      style={{
        width: "100%",
        padding: 32,
        backgroundColor: "#1a1a2e",
        justifyContent: "center",
        alignItems: "center",
        marginBottom: 24,
      }}
    >
      <Text
        style={{
          fontSize: 36,
          color: "#ffffff",
        }}
      >
        Bevy + React Demo
      </Text>
      <Text
        style={{
          fontSize: 16,
          color: "#888888",
          marginTop: 8,
        }}
      >
        Interactive UI Components
      </Text>
    </Node>
  )
}

// Counter card component with increment/decrement buttons
function CounterCard() {
  const [count, setCount] = useState(0)

  return (
    <Node
      style={{
        width: "80%",
        maxWidth: 400,
        padding: 24,
        backgroundColor: "#16213e",
        borderRadius: 12,
        borderWidth: 2,
        borderColor: "#0f3460",
        flexDirection: "column",
        alignItems: "center",
        marginBottom: 24,
      }}
    >
      <Text
        style={{
          fontSize: 20,
          color: "#e94560",
          marginBottom: 16,
        }}
      >
        Counter
      </Text>
      
      <Text
        style={{
          fontSize: 48,
          color: "#ffffff",
          marginBottom: 24,
          fontFamily: "monospace",
        }}
      >
        {count}
      </Text>

      <Node
        style={{
          flexDirection: "row",
          alignItems: "center",
        }}
      >
        <Button
          onClick={() => setCount((c) => Math.max(0, c - 1))}
          style={{
            padding: 12,
            paddingLeft: 24,
            paddingRight: 24,
            backgroundColor: "#e94560",
            borderRadius: 8,
            minWidth: 100,
            justifyContent: "center",
            alignItems: "center",
            marginRight: 16,
          }}
        >
          <Text
            style={{
              fontSize: 18,
              color: "#ffffff",
            }}
          >
            Decrement
          </Text>
        </Button>

        <Button
          onClick={() => setCount((c) => c + 1)}
          style={{
            padding: 12,
            paddingLeft: 24,
            paddingRight: 24,
            backgroundColor: "#0f3460",
            borderRadius: 8,
            minWidth: 100,
            justifyContent: "center",
            alignItems: "center",
          }}
        >
          <Text
            style={{
              fontSize: 18,
              color: "#ffffff",
            }}
          >
            Increment
          </Text>
        </Button>
      </Node>

      <Button
        onClick={() => setCount(0)}
        style={{
          padding: 8,
          paddingLeft: 16,
          paddingRight: 16,
          backgroundColor: "#2a2a3a",
          borderRadius: 6,
          marginTop: 16,
          borderWidth: 1,
          borderColor: "#444444",
        }}
      >
        <Text
          style={{
            fontSize: 14,
            color: "#aaaaaa",
          }}
        >
          Reset
        </Text>
      </Button>
    </Node>
  )
}

// Text input card component
function TextInputCard() {
  const [inputValue, setInputValue] = useState("")

  return (
    <Node
      style={{
        width: "80%",
        maxWidth: 400,
        padding: 24,
        backgroundColor: "#16213e",
        borderRadius: 12,
        borderWidth: 2,
        borderColor: "#0f3460",
        flexDirection: "column",
        alignItems: "center",
      }}
    >
      <Text
        style={{
          fontSize: 20,
          color: "#e94560",
          marginBottom: 16,
        }}
      >
        Text Input
      </Text>

      <TextInput
        value={inputValue}
        onChange={setInputValue}
        placeholder="Type something here..."
        style={{
          width: "100%",
          backgroundColor: "#1a1a2e",
          borderWidth: 2,
          borderColor: "#0f3460",
          borderRadius: 8,
          padding: 12,
          marginBottom: 16,
        }}
        textStyle={{
          fontSize: 16,
          color: "#ffffff",
        }}
        placeholderStyle={{
          fontSize: 16,
          color: "#666666",
        }}
      />

      {inputValue && (
        <Node
          style={{
            width: "100%",
            padding: 12,
            backgroundColor: "#0f3460",
            borderRadius: 6,
            marginTop: 8,
          }}
        >
          <Text
            style={{
              fontSize: 14,
              color: "#aaaaaa",
              marginBottom: 4,
            }}
          >
            You typed:
          </Text>
          <Text
            style={{
              fontSize: 18,
              color: "#ffffff",
              fontFamily: "monospace",
            }}
          >
            "{inputValue}"
          </Text>
          <Text
            style={{
              fontSize: 12,
              color: "#888888",
              marginTop: 8,
            }}
          >
            Length: {inputValue.length} characters
          </Text>
        </Node>
      )}
    </Node>
  )
}

// Main App component
function App() {
  return (
    <Node
      style={{
        width: "100%",
        height: "100%",
        backgroundColor: "#0f0f1e",
        flexDirection: "column",
        alignItems: "center",
        paddingTop: 0,
        paddingBottom: 40,
        overflow: "visible",
      }}
    >
      <Header />
      <CounterCard />
      <TextInputCard />
    </Node>
  )
}

export default App
