export default function ReactorSection() {
  return (
    <section className="reactor">
      <div className="reactor-box">
        <div className="reactor-header">
          <div className="section-label">How it flows</div>
        </div>
        <div className="reactor-flow">
          <div className="reactor-node">
            <div className="reactor-core">
              <div className="reactor-core-inner" />
            </div>
            <span className="reactor-label">Deposit</span>
            <span className="reactor-sub">Wallet</span>
          </div>
          <div className="reactor-line" />
          <div className="reactor-node">
            <div className="reactor-core">
              <div className="reactor-core-inner" />
            </div>
            <span className="reactor-label">Execute</span>
            <span className="reactor-sub">Bot</span>
          </div>
          <div className="reactor-line" />
          <div className="reactor-node">
            <div className="reactor-core">
              <div className="reactor-core-inner" />
            </div>
            <span className="reactor-label">Exchange</span>
            <span className="reactor-sub">Market</span>
          </div>
          <div className="reactor-line" />
          <div className="reactor-node">
            <div className="reactor-core">
              <div className="reactor-core-inner" />
            </div>
            <span className="reactor-label">Stake</span>
            <span className="reactor-sub">Pool</span>
          </div>
          <div className="reactor-line" />
          <div className="reactor-node">
            <div className="reactor-core">
              <div className="reactor-core-inner" />
            </div>
            <span className="reactor-label">Withdraw</span>
            <span className="reactor-sub">Profit</span>
          </div>
        </div>
      </div>
    </section>
  )
}
