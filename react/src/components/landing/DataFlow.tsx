/**
 * Decorative distributed-system data-flow: service nodes with light packets
 * travelling along the connections between them. Pure SVG (animateMotion +
 * mpath), no JS. Meant to sit behind the hero's right-hand area.
 */
const NODES = [
  { id: 'wallet', label: 'Wallet', x: 78, y: 66 },
  { id: 'api', label: 'API', x: 78, y: 200 },
  { id: 'redis', label: 'Redis', x: 205, y: 140 },
  { id: 'analyzer', label: 'Analyzer', x: 335, y: 80 },
  { id: 'db', label: 'DB', x: 205, y: 320 },
  { id: 'engine', label: 'Engine', x: 335, y: 235 },
  { id: 'kucoin', label: 'KuCoin', x: 365, y: 370 },
] as const

const EDGES: [string, string][] = [
  ['wallet', 'api'],
  ['api', 'redis'],
  ['api', 'db'],
  ['redis', 'analyzer'],
  ['redis', 'engine'],
  ['engine', 'db'],
  ['engine', 'kucoin'],
]

const pos = (id: string) => NODES.find(n => n.id === id)!

export default function DataFlow({ className = '' }: { className?: string }) {
  return (
    <svg
      className={`pointer-events-none select-none ${className}`}
      viewBox="0 0 440 440"
      fill="none"
      preserveAspectRatio="xMidYMid meet"
      aria-hidden="true"
      data-testid="data-flow"
    >
      <defs>
        <filter id="df-glow" x="-200%" y="-200%" width="500%" height="500%">
          <feGaussianBlur stdDeviation="3" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* links + moving packets */}
      {EDGES.map(([a, b], i) => {
        const pa = pos(a)
        const pb = pos(b)
        const d = `M${pa.x},${pa.y} L${pb.x},${pb.y}`
        return (
          <g key={`${a}-${b}`}>
            <path id={`df-e${i}`} d={d} stroke="rgba(255,255,255,0.09)" strokeWidth="1.2" />
            <circle r="3" fill="var(--color-accent)" filter="url(#df-glow)">
              <animateMotion dur={`${2.4 + (i % 3) * 0.6}s`} begin={`${(i * 0.4).toFixed(1)}s`} repeatCount="indefinite">
                <mpath href={`#df-e${i}`} />
              </animateMotion>
            </circle>
          </g>
        )
      })}

      {/* nodes */}
      {NODES.map(n => (
        <g key={n.id}>
          <circle cx={n.x} cy={n.y} r="15" fill="rgba(255,87,34,0.06)" stroke="rgba(255,87,34,0.35)" strokeWidth="1" />
          <circle cx={n.x} cy={n.y} r="3" fill="var(--color-accent)" filter="url(#df-glow)" />
          <text
            x={n.x}
            y={n.y + 27}
            textAnchor="middle"
            fontSize="8.5"
            letterSpacing="0.12em"
            fill="rgba(255,255,255,0.5)"
            style={{ textTransform: 'uppercase', fontFamily: 'var(--font-mono)' }}
          >
            {n.label}
          </text>
        </g>
      ))}
    </svg>
  )
}
