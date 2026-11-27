import * as React from 'react'
import { Check, Copy } from 'lucide-react'
import { cn } from '@/lib/utils'

/** A block of text (prompt or command) with a copy button. */
export function CopyBlock({ text, label, mono = true, className }: { text: string; label?: string; mono?: boolean; className?: string }) {
  const [done, setDone] = React.useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setDone(true)
      setTimeout(() => setDone(false), 1500)
    } catch {
      /* clipboard blocked: the text stays selectable */
    }
  }
  return (
    <div className={cn('group relative rounded-lg border bg-muted/50', className)}>
      {label && <div className="border-b px-3 py-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{label}</div>}
      <pre className={cn('overflow-x-auto whitespace-pre-wrap break-words px-3 py-2.5 pr-12 text-[13px] leading-relaxed', mono ? 'font-mono' : 'font-sans')}>{text}</pre>
      <button
        onClick={copy}
        className="absolute right-2 bottom-2 inline-flex items-center gap-1 rounded-md border bg-card px-2 py-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
        aria-label="Copy"
      >
        {done ? <Check className="size-3.5 text-success" /> : <Copy className="size-3.5" />}
        {done ? 'Copied' : 'Copy'}
      </button>
    </div>
  )
}
