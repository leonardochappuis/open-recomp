import * as React from 'react'
import { Slot } from '@radix-ui/react-slot'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-1.5 rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50 cursor-pointer',
  {
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground hover:bg-primary/90',
        outline: 'border bg-card hover:bg-accent',
        ghost: 'hover:bg-accent text-muted-foreground hover:text-foreground',
        chip: 'h-7 rounded-full border bg-card px-3 text-xs text-muted-foreground hover:text-foreground hover:border-primary/50 data-[on=true]:border-primary data-[on=true]:bg-primary data-[on=true]:text-primary-foreground',
      },
      size: { default: 'h-9 px-3.5', sm: 'h-8 px-2.5 text-xs', icon: 'size-9' },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  },
)

export function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<'button'> & VariantProps<typeof buttonVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot : 'button'
  return <Comp className={cn(buttonVariants({ variant, size: variant === 'chip' ? null : size }), className)} {...props} />
}
