import { useEffect, useState } from 'react'

export type Route = { path: string; params: URLSearchParams }

function parse(): Route {
  const h = window.location.hash.slice(1)
  const i = h.indexOf('?')
  return { path: (i < 0 ? h : h.slice(0, i)) || '/', params: new URLSearchParams(i < 0 ? '' : h.slice(i + 1)) }
}

export function useRoute(): Route {
  const [r, setR] = useState(parse)
  useEffect(() => {
    const on = () => { setR(parse()); }
    window.addEventListener('hashchange', on)
    window.addEventListener('orc:route', on)
    return () => { window.removeEventListener('hashchange', on); window.removeEventListener('orc:route', on) }
  }, [])
  return r
}

/** Update query params without adding history entries (filters, search). */
export function setParams(path: string, params: URLSearchParams) {
  const q = params.toString()
  history.replaceState(null, '', `#${path}${q ? '?' + q : ''}`)
  window.dispatchEvent(new Event('orc:route'))
}

export function withParam(route: Route, key: string, value: string | null) {
  const p = new URLSearchParams(route.params)
  if (value) p.set(key, value)
  else p.delete(key)
  setParams(route.path, p)
}
