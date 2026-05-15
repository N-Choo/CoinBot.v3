import { useEffect, useRef, memo } from 'react'

interface TradingChartProps {
  symbol: string
}

const TradingChart = memo<TradingChartProps>(({ symbol }) => {
  const container = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!container.current) return
    container.current.innerHTML = ''

    const script = document.createElement('script')
    script.src = 'https://s3.tradingview.com/external-embedding/embed-widget-advanced-chart.js'
    script.type = 'text/javascript'
    script.async = true

    const formattedSymbol = `BINANCE:${symbol.replace('/', '')}`

    script.innerHTML = JSON.stringify({
      autosize: true,
      symbol: formattedSymbol,
      interval: '60',
      timezone: 'Etc/UTC',
      theme: 'dark',
      style: '1',
      locale: 'en',
      enable_publishing: false,
      hide_top_toolbar: false,
      hide_legend: false,
      save_image: false,
      container_id: 'tradingview_chart_container'
    })

    container.current.appendChild(script)
  }, [symbol])

  return (
    <div className="tradingview-widget-container" ref={container} style={{ height: '100%', width: '100%' }}>
      <div id="tradingview_chart_container" style={{ height: '100%', width: '100%' }} />
    </div>
  )
})

TradingChart.displayName = 'TradingChart'

export default TradingChart
