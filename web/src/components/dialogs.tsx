import * as React from 'react'
import { Bot, CheckCircle2, ExternalLink, FileText, GitFork, Plus, Terminal, Users } from 'lucide-react'
import { platName, type Game } from '@/lib/data'
import { CopyBlock } from '@/components/copy'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

const guide = (hub: string, file: string) => `https://github.com/${hub}/blob/main/docs/agents/${file}`

function Steps({ items }: { items: React.ReactNode[] }) {
  return (
    <ol className="space-y-3">
      {items.map((it, i) => (
        <li key={i} className="flex gap-3 text-sm">
          <span className="grid size-6 shrink-0 place-items-center rounded-full bg-primary/15 text-xs font-semibold text-primary">{i + 1}</span>
          <div className="min-w-0 pt-0.5">{it}</div>
        </li>
      ))}
    </ol>
  )
}

export function AddGameDialog({ hub, trigger }: { hub: string; trigger?: React.ReactNode }) {
  const form = `https://github.com/${hub}/issues/new?template=add-game.yml`
  const prompt = `Read ${guide(hub, 'add-a-game.md')} and help me add my game to open-recomp. Ask me for anything you need.`
  return (
    <Dialog>
      <DialogTrigger asChild>
        {trigger ?? (
          <Button>
            <Plus className="size-4" /> Add your game
          </Button>
        )}
      </DialogTrigger>
      <DialogContent>
        <DialogTitle>Add your game</DialogTitle>
        <DialogDescription>
          List a recompilation project so others can find it, follow it and pitch in. A maintainer reviews every entry; until then it shows under Unfiltered.
        </DialogDescription>
        <Tabs defaultValue="agent" className="mt-5">
          <TabsList>
            <TabsTrigger value="agent"><Bot className="size-4" /> With your agent</TabsTrigger>
            <TabsTrigger value="form"><FileText className="size-4" /> Fill a form</TabsTrigger>
            <TabsTrigger value="cli"><Terminal className="size-4" /> With orc</TabsTrigger>
          </TabsList>
          <TabsContent value="agent" className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Paste this into Claude Code, Codex or any coding agent. It reads the guide, asks you for the details and gives you a prefilled form to submit.
            </p>
            <CopyBlock text={prompt} label="Prompt" mono={false} />
          </TabsContent>
          <TabsContent value="form" className="space-y-4">
            <Steps
              items={[
                <>Open the form on GitHub (you need a GitHub account) and fill in what you know. Drag the box art into the cover field.</>,
                <>Your game shows up right away under <a href="#/unfiltered" className="text-primary">Unfiltered</a>, marked as unreviewed.</>,
                <>A maintainer adds the <code className="rounded bg-muted px-1 font-mono text-xs">approved</code> label and it moves into the catalog automatically.</>,
              ]}
            />
            <Button asChild>
              <a href={form} target="_blank" rel="noopener noreferrer">
                Open the form <ExternalLink className="size-4" />
              </a>
            </Button>
          </TabsContent>
          <TabsContent value="cli" className="space-y-4">
            <p className="text-sm text-muted-foreground">If you already use orc, this opens a pull request with the entry and cover:</p>
            <CopyBlock
              label="Terminal"
              text={`orc game add --platform x360 --id 4D5307E6 --name "Original game" --title "Recomp name" \\\n  --status in-progress --target windows --target linux \\\n  --repo you/your-recomp --cover-file cover.png`}
            />
          </TabsContent>
        </Tabs>
        <p className="mt-5 text-xs text-muted-foreground">Never attach ROMs, ISOs or game files. Box art you're allowed to share is fine.</p>
      </DialogContent>
    </Dialog>
  )
}

export function ContributeDialog({ hub, game }: { hub: string; game: Game }) {
  const title = game.title || game.name
  const repo = game.repos?.[0]
  const prompt =
    `Read ${guide(hub, 'contribute.md')} and set me up to contribute to ${title} ` +
    `(${platName(game.platform)}, open-recomp scope ${game.scope}${repo ? `, recomp repo ${repo}` : ''}). ` +
    `Then tell me what is being worked on right now and where I could help.`
  const manual = [
    `cargo install --locked --git https://github.com/${hub} orc`,
    `gh auth login`,
    repo ? `gh repo fork ${repo} --clone && cd ${repo.split('/')[1]}` : `# clone the recomp's repository`,
    `orc init --name "${title.replace(/"/g, '')}" --kb ${hub} --scope ${game.scope}   # only if it has no .orc/ yet`,
    `orc check "what you want to work on"`,
  ].join('\n')
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button className="bg-glow text-zinc-950 hover:bg-glow/90">
          <Users className="size-4" /> Contribute
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogTitle>Contribute to {title}</DialogTitle>
        <DialogDescription>Your agent works next to everyone else's: it sees what others already found and what they're trying right now.</DialogDescription>
        <Tabs defaultValue="agent" className="mt-5">
          <TabsList>
            <TabsTrigger value="agent"><Bot className="size-4" /> With your agent</TabsTrigger>
            <TabsTrigger value="manual"><Terminal className="size-4" /> By hand</TabsTrigger>
            <TabsTrigger value="how"><CheckCircle2 className="size-4" /> How it works</TabsTrigger>
          </TabsList>
          <TabsContent value="agent" className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Open Claude Code or Codex in an empty folder and paste this. It installs what's needed (asking you first), clones the project and connects it.
            </p>
            <CopyBlock text={prompt} label="Prompt" mono={false} />
            {repo && (
              <Button variant="outline" asChild>
                <a href={`https://github.com/${repo}`} target="_blank" rel="noopener noreferrer">
                  <GitFork className="size-4" /> {repo}
                </a>
              </Button>
            )}
          </TabsContent>
          <TabsContent value="manual" className="space-y-4">
            <CopyBlock text={manual} label="Terminal" />
            <p className="text-sm text-muted-foreground">
              Then open your agent in that folder. You'll need your own legally obtained copy of the game; nothing here ever hosts game files.
            </p>
          </TabsContent>
          <TabsContent value="how">
            <Steps
              items={[
                <><b>Before you start</b>, your agent checks who is already on the problem and on what hardware, and tells you, so you don't duplicate work.</>,
                <><b>Attempts aren't locks.</b> Several people can try different approaches on the same bug; each one shows up live on this page.</>,
                <><b>Everything learned is shared</b>, including what didn't work, with evidence and the hardware it was seen on (GPU vendor, OS; never your name or machine details).</>,
                <><b>Findings get verified</b> when someone else reproduces them on their own machine. Reproducing is one of the most useful things you can do.</>,
              ]}
            />
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
