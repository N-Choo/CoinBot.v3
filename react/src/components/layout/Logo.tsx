export function LogoMark({ className = '' }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={`relative inline-block shrink-0 rounded-[6px] bg-gradient-to-br from-[#ff8a65] to-[#ff5722] shadow-[0_0_18px_rgba(255,87,34,0.35)] ${className}`}
    >
      <span className="absolute inset-[3px] rounded-[3px] border border-white/25" />
    </span>
  )
}

export default function Logo() {
  return (
    <span className="flex items-center gap-2.5">
      <LogoMark className="h-[22px] w-[22px]" />
      <span className="text-text-main font-mono font-extrabold text-lg sm:text-lg xs:text-base tracking-[0.5px]">CoinBot</span>
    </span>
  )
}
