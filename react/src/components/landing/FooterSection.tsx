export default function FooterSection() {
  return (
    <footer className="text-center py-6 px-4 sm:px-6 border-t border-border-light text-xs text-text-muted">
      <div className="max-w-[var(--max-width,1200px)] mx-auto flex flex-col sm:flex-row justify-between items-center gap-1.5">
        <span className="bg-gradient-to-r from-accent to-[#ff8a65] bg-clip-text text-transparent font-bold font-mono">CoinBot</span>
        <p className="m-0">&copy; {new Date().getFullYear()} &mdash; All rights reserved.</p>
      </div>
    </footer>
  )
}
