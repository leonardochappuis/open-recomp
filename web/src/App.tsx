import { useEffect } from 'react'
import { useDashboard, useTick } from '@/lib/data'
import { useRoute } from '@/lib/router'
import { Footer, Header } from '@/components/layout'
import { EmptyState } from '@/components/common'
import { Skeleton } from '@/components/ui/skeleton'
import { TooltipProvider } from '@/components/ui/tooltip'
import { Live } from '@/views/Live'
import { Games } from '@/views/Games'
import { GameView } from '@/views/Game'
import { Attempts } from '@/views/Attempts'
import { Findings } from '@/views/Findings'
import { FindingView } from '@/views/FindingView'

function Loading() {
  return (
    <div className="space-y-6">
      <Skeleton className="h-10 w-2/3" />
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">{Array.from({ length: 5 }, (_, i) => <Skeleton key={i} className="h-24" />)}</div>
      <div className="grid gap-6 lg:grid-cols-2"><Skeleton className="h-96" /><Skeleton className="h-96" /></div>
    </div>
  )
}

export default function App() {
  const { data, fresh, error } = useDashboard()
  const route = useRoute()
  useTick()
  useEffect(() => { window.scrollTo(0, 0) }, [route.path])

  const p = route.path
  const nav = p.startsWith('/game') ? 'games' : p.startsWith('/attempts') ? 'attempts' : p.startsWith('/finding') ? 'findings' : 'live'

  let view: React.ReactNode
  if (!data) view = error ? <EmptyState title="Could not load data">The dashboard data is not available yet. Try again in a minute.</EmptyState> : <Loading />
  else if (p === '/') view = <Live data={data} fresh={fresh} />
  else if (p === '/games') view = <Games data={data} route={route} />
  else if (p.startsWith('/game/')) view = <GameView data={data} route={route} scope={decodeURIComponent(p.slice(6))} />
  else if (p === '/attempts') view = <Attempts data={data} route={route} />
  else if (p === '/findings') view = <Findings data={data} route={route} />
  else if (p.startsWith('/finding/')) {
    const parts = p.slice(9).split('/').map(decodeURIComponent)
    const id = parts.pop() ?? ''
    view = <FindingView data={data} repo={parts.join('/')} id={id} />
  } else view = <EmptyState title="Page not found"><a href="#/" className="text-primary">Back to live</a></EmptyState>

  return (
    <TooltipProvider>
      <div className="page-glow flex min-h-screen flex-col">
        <Header data={data} error={error} active={nav} />
        <main className="mx-auto w-full max-w-7xl flex-1 px-4 py-8 sm:px-6">{view}</main>
        <Footer />
      </div>
    </TooltipProvider>
  )
}
