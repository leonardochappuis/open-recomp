import * as React from 'react'
import { Inbox } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Tooltip } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { ago, hueOf, inScope, isLogin, platName, platShort, safeUrl, STATUS_LABEL, type Data, type Game, type Hw } from '@/lib/data'

export function Avatar({ login, size = 24, className }: { login: string; size?: number; className?: string }) {
  const [loaded, setLoaded] = React.useState(false)
  const [broken, setBroken] = React.useState(false)
  const ok = isLogin(login)
  const initials = login.replace(/[^a-z0-9]/gi, '').slice(0, 2).toUpperCase() || '?'
  return (
    <span
      style={{ width: size, height: size, background: `oklch(0.55 0.12 ${hueOf(login)})` }}
      className={cn('relative inline-grid shrink-0 place-items-center overflow-hidden rounded-full text-[10px] font-semibold text-white ring-1 ring-border', className)}
    >
      {initials}
      {ok && !broken && (
        <img
          src={`https://github.com/${login}.png?size=${size * 2}`}
          alt=""
          loading="lazy"
          referrerPolicy="no-referrer"
          onLoad={() => setLoaded(true)}
          onError={() => setBroken(true)}
          className={cn('absolute inset-0 size-full object-cover transition-opacity', loaded ? 'opacity-100' : 'opacity-0')}
        />
      )}
    </span>
  )
}

export function UserLink({ login, className }: { login: string; className?: string }) {
  if (!isLogin(login)) return <span className={cn('font-medium', className)}>{login}</span>
  return (
    <a href={`https://github.com/${login}`} target="_blank" rel="noopener noreferrer" className={cn('font-medium hover:underline', className)}>
      {login}
    </a>
  )
}

export function TimeAgo({ iso, className }: { iso: string; className?: string }) {
  const d = new Date(iso)
  return (
    <time dateTime={iso} title={isNaN(+d) ? '' : d.toLocaleString()} className={cn('tabular-nums text-muted-foreground', className)}>
      {ago(iso)}
    </time>
  )
}

export function ExtLink({ href, children, className }: { href: unknown; children: React.ReactNode; className?: string }) {
  const u = safeUrl(href)
  if (!u) return <span className={className}>{children}</span>
  return (
    <a href={u} target="_blank" rel="noopener noreferrer" className={cn('hover:underline', className)}>
      {children}
    </a>
  )
}

const VENDOR: Record<string, { label: string; dot: string }> = {
  nvidia: { label: 'NVIDIA', dot: 'bg-[oklch(0.72_0.19_135)]' },
  amd: { label: 'AMD', dot: 'bg-[oklch(0.62_0.22_27)]' },
  intel: { label: 'Intel', dot: 'bg-[oklch(0.62_0.15_245)]' },
  apple: { label: 'Apple', dot: 'bg-[oklch(0.75_0.01_260)]' },
  qualcomm: { label: 'Qualcomm', dot: 'bg-[oklch(0.6_0.15_265)]' },
  microsoft: { label: 'Virtual', dot: 'bg-[oklch(0.6_0.02_260)]' },
}
const OS: Record<string, string> = { linux: 'Linux', windows: 'Windows', macos: 'macOS' }

export function HardwareChips({ hw, compact }: { hw?: Hw | null; compact?: boolean }) {
  if (!hw) return <span className="text-xs text-muted-foreground">hardware unknown</span>
  const vendors = hw.gpu_vendors.length ? hw.gpu_vendors : ['unknown']
  return (
    <Tooltip content={hw.summary}>
      <span className="inline-flex items-center gap-1 cursor-default">
        {vendors.map((v) => (
          <span key={v} className="inline-flex items-center gap-1.5 rounded-full border bg-card px-2 py-0.5 text-[11px] font-medium">
            <span className={cn('size-1.5 rounded-full', VENDOR[v]?.dot ?? 'bg-muted-foreground')} />
            {VENDOR[v]?.label ?? v}
          </span>
        ))}
        {!compact && (
          <span className="rounded-full border bg-card px-2 py-0.5 text-[11px] font-medium text-muted-foreground">{OS[hw.os] ?? hw.os}</span>
        )}
      </span>
    </Tooltip>
  )
}

export function FindingStatus({ status }: { status: string }) {
  const tone = ({ verified: 'success', reproduced: 'info', proposed: 'warning', disputed: 'danger', retracted: 'neutral' } as const)[status as 'verified'] ?? 'neutral'
  return <Badge tone={tone}>{status}</Badge>
}

export function GameStatus({ status }: { status: string }) {
  const tone = ({ complete: 'success', playable: 'info', boots: 'primary', 'in-progress': 'warning', planned: 'neutral' } as const)[status as 'complete'] ?? 'neutral'
  return <Badge tone={tone}>{STATUS_LABEL[status] ?? status}</Badge>
}

export function LiveDot({ className, tone = 'success' }: { className?: string; tone?: 'success' | 'warning' | 'danger' }) {
  const color = { success: 'bg-success', warning: 'bg-warning', danger: 'bg-danger' }[tone]
  return (
    <span className={cn('relative inline-flex size-2', className)}>
      {tone === 'success' && <span className={cn('absolute inline-flex size-full animate-ping rounded-full opacity-60', color)} />}
      <span className={cn('relative inline-flex size-2 rounded-full', color)} />
    </span>
  )
}

export function Cover({ game, live = 0, className, large }: { game: Game; live?: number; className?: string; large?: boolean }) {
  const url = safeUrl(game.cover)
  const [broken, setBroken] = React.useState(false)
  const hue = hueOf(game.name + game.platform)
  const initials = game.name.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join('').toUpperCase()
  return (
    <div className={cn('relative aspect-[3/4] overflow-hidden bg-muted', className)}>
      {url && !broken ? (
        <img src={url} alt="" loading="lazy" referrerPolicy="no-referrer" onError={() => setBroken(true)} className="size-full object-cover" />
      ) : (
        <div
          className="grid size-full place-items-center"
          style={{
            background: `radial-gradient(120% 90% at 20% 10%, oklch(0.62 0.17 ${hue}) 0%, oklch(0.38 0.14 ${(hue + 40) % 360}) 55%, oklch(0.22 0.06 ${(hue + 80) % 360}) 100%)`,
          }}
        >
          <span className={cn('font-bold tracking-tight text-white/90 drop-shadow', large ? 'text-5xl' : 'text-4xl')}>{initials}</span>
        </div>
      )}
      <div className="pointer-events-none absolute inset-x-0 top-0 flex items-start justify-between p-2">
        <span className="rounded-md bg-black/55 px-1.5 py-0.5 text-[10px] font-semibold tracking-wide text-white backdrop-blur">
          {platShort(game.platform)}
        </span>
        {live > 0 && (
          <span className="inline-flex items-center gap-1.5 rounded-md bg-black/55 px-1.5 py-0.5 text-[10px] font-semibold text-white backdrop-blur">
            <LiveDot className="size-1.5 [&>span]:size-1.5" /> {live} live
          </span>
        )}
      </div>
    </div>
  )
}

export function ScopeLabel({ scope, data, className }: { scope: string; data: Data; className?: string }) {
  const g = data.games.find((x) => inScope(scope, x.scope))
  if (g) {
    const rest = scope.slice(g.scope.length)
    return (
      <a href={`#/game/${encodeURIComponent(g.scope)}`} className={cn('inline-flex min-w-0 items-baseline gap-1 hover:underline', className)}>
        <span className="truncate font-medium text-foreground/90">{g.name}</span>
        {rest && <span className="truncate font-mono text-[11px] text-muted-foreground">{rest}</span>}
      </a>
    )
  }
  return <span className={cn('truncate font-mono text-[11px] text-muted-foreground', className)}>{scope}</span>
}

export function EmptyState({ icon: Icon = Inbox, title, children }: { icon?: React.ElementType; title: string; children?: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed px-6 py-10 text-center">
      <div className="grid size-10 place-items-center rounded-full bg-muted text-muted-foreground">
        <Icon className="size-5" />
      </div>
      <div className="font-medium">{title}</div>
      {children && <div className="max-w-md text-sm text-muted-foreground">{children}</div>}
    </div>
  )
}

export function PageHeader({ title, description, children }: { title: React.ReactNode; description?: React.ReactNode; children?: React.ReactNode }) {
  return (
    <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div className="min-w-0">
        <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">{title}</h1>
        {description && <p className="mt-1 text-sm text-muted-foreground">{description}</p>}
      </div>
      {children}
    </div>
  )
}

export function platformTitle(p: string) {
  return platName(p)
}
