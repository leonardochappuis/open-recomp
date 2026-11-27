import { Apple, ArrowDown, ArrowUp, Globe, Smartphone, Terminal, Zap } from 'lucide-react'
import { cn } from '@/lib/utils'
import { platColor, platName, type Game } from '@/lib/data'
import type { Trend } from '@/lib/trending'
import { Cover } from '@/components/common'
import { Tooltip } from '@/components/ui/tooltip'

function WindowsMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden className={className}>
      <path d="M0 2.2 6.5 1.3v6.2H0zM7.3 1.2 16 0v7.5H7.3zM0 8.3h6.5v6.3L0 13.7zM7.3 8.3H16V16l-8.7-1.2z" />
    </svg>
  )
}

const TARGET: Record<string, { label: string; icon: React.ElementType }> = {
  windows: { label: 'Windows', icon: WindowsMark },
  linux: { label: 'Linux', icon: Terminal },
  macos: { label: 'macOS', icon: Apple },
  android: { label: 'Android', icon: Smartphone },
  web: { label: 'Browser', icon: Globe },
}

export function Targets({ targets, className }: { targets?: string[]; className?: string }) {
  const list = (targets ?? []).filter((t) => TARGET[t])
  if (!list.length) return null
  return (
    <Tooltip content={`Runs on ${list.map((t) => TARGET[t].label).join(', ')}`}>
      <span className={cn('inline-flex items-center gap-1 rounded-md bg-black/60 px-1.5 py-1 text-white backdrop-blur', className)}>
        {list.map((t) => {
          const Icon = TARGET[t].icon
          return <Icon key={t} className="size-3" />
        })}
      </span>
    </Tooltip>
  )
}

export function ConsoleLabel({ platform, className }: { platform: string; className?: string }) {
  return (
    <span className={cn('inline-flex items-center gap-1.5', className)}>
      <span className="size-2 rounded-full" style={{ background: platColor(platform) }} />
      {platName(platform)}
    </span>
  )
}

function RankBadge({ rank }: { rank: number }) {
  const podium = rank <= 3
  return (
    <span
      className={cn(
        'grid h-7 min-w-7 place-items-center rounded-lg px-1.5 text-sm font-bold tabular-nums shadow-md',
        podium ? 'bg-gradient-to-b from-amber-200 to-amber-400 text-amber-950' : 'bg-white/90 text-zinc-900',
      )}
    >
      {rank}
    </span>
  )
}

function TrendChip({ t }: { t: Trend }) {
  if (t.rank && t.prevRank && t.prevRank !== t.rank) {
    const up = t.prevRank > t.rank
    const n = Math.abs(t.prevRank - t.rank)
    return (
      <Tooltip content={`${up ? 'Up' : 'Down'} ${n} from last week`}>
        <span className={cn('inline-flex items-center gap-0.5 rounded-md bg-black/60 px-1.5 py-1 text-[11px] font-semibold backdrop-blur', up ? 'text-emerald-400' : 'text-rose-400')}>
          {up ? <ArrowUp className="size-3" /> : <ArrowDown className="size-3" />}
          {n}
        </span>
      </Tooltip>
    )
  }
  if (t.isNew) return <span className="rounded-md bg-black/60 px-1.5 py-1 text-[10px] font-bold tracking-wide text-amber-300 backdrop-blur">NEW</span>
  return null
}

export type TileGame = Game & { live: number; findings: number; verified: number }

/** Cover card in the style of a chart: rank, movement, targets, live work. */
export function GameTile({ g, trend, showRank, className }: { g: TileGame; trend?: Trend; showRank?: boolean; className?: string }) {
  // Movement only means something next to a rank; elsewhere just flag new entries.
  const chip = trend && (showRank ? trend : trend.isNew && !trend.prevRank ? { ...trend, rank: undefined } : undefined)
  const title = g.title || g.name
  return (
    <a href={`#/game/${encodeURIComponent(g.scope)}`} className={cn('group block outline-none', className)}>
      <div
        className={cn(
          'relative overflow-hidden rounded-2xl ring-1 ring-border transition-all duration-200',
          'group-hover:-translate-y-1 group-hover:ring-2 group-hover:ring-glow group-hover:shadow-[0_0_28px_-6px_var(--glow)]',
          'group-focus-visible:ring-2 group-focus-visible:ring-glow',
        )}
      >
        <Cover game={g} bare className="transition-transform duration-500 group-hover:scale-[1.04]" />
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/75 via-black/0 via-45% to-black/30" />

        <div className="absolute inset-x-2 top-2 flex items-start justify-between gap-1">
          <div className="flex items-center gap-1">
            {showRank && trend?.rank ? <RankBadge rank={trend.rank} /> : null}
            {chip && <TrendChip t={chip} />}
          </div>
          <Targets targets={g.targets} />
        </div>

        <div className="absolute inset-x-2 bottom-2 flex items-center gap-1.5">
          {g.live > 0 && (
            <Tooltip content={`${g.live} attempt${g.live === 1 ? '' : 's'} in progress right now`}>
              <span className="grid size-7 place-items-center rounded-full bg-glow text-zinc-950 shadow-[0_0_14px_var(--glow)]">
                <Zap className="size-3.5 fill-current" />
              </span>
            </Tooltip>
          )}
          <span className="inline-flex items-center gap-1.5 rounded-full bg-black/60 px-2.5 py-1 text-[11px] font-medium text-white backdrop-blur">
            <span className="size-1.5 rounded-full bg-glow" />
            {g.findings} finding{g.findings === 1 ? '' : 's'}
          </span>
        </div>
      </div>
      <div className="mt-2.5 space-y-0.5 text-center">
        <div className="truncate text-sm font-semibold">{title}</div>
        {g.title && <div className="truncate text-xs text-muted-foreground">{g.name}</div>}
        <ConsoleLabel platform={g.platform} className="justify-center text-xs text-muted-foreground" />
      </div>
    </a>
  )
}
