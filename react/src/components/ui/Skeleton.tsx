interface SkeletonProps {
  variant?: 'text' | 'text-sm' | 'text-lg' | 'circle' | 'chart' | 'card'
  width?: string
  height?: string
  className?: string
}

const sizeMap: Record<string, string> = {
  text: 'skeleton-text',
  'text-sm': 'skeleton-text-sm',
  'text-lg': 'skeleton-text-lg',
  circle: 'skeleton-circle',
  chart: 'skeleton-chart',
  card: 'skeleton-card',
}

export default function Skeleton({ variant = 'text', width, height, className = '' }: SkeletonProps) {
  return (
    <div
      className={`skeleton ${sizeMap[variant] || ''} ${className}`}
      style={{ width, height }}
    />
  )
}
