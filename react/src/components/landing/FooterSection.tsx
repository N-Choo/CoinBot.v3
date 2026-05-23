export default function FooterSection() {
  return (
    <footer className="footer">
      <div className="footer-inner">
        <span className="footer-brand">CoinBot</span>
        <p>&copy; {new Date().getFullYear()} &mdash; All rights reserved.</p>
      </div>
    </footer>
  )
}
