import * as React from "react"

// In the Tauri shell, this hook represents compact navigation mode rather
// than a phone. The app cannot be resized below 800px, so the old 768px
// threshold was unreachable and left the workspace cramped at small windows.
const COMPACT_NAV_BREAKPOINT = 1024

export function useIsMobile() {
  const [isMobile, setIsMobile] = React.useState<boolean | undefined>(undefined)

  React.useEffect(() => {
    const query = `(max-width: ${COMPACT_NAV_BREAKPOINT - 1}px)`
    const mql = window.matchMedia(query)
    const onChange = () => setIsMobile(mql.matches)

    mql.addEventListener("change", onChange)
    onChange()
    return () => mql.removeEventListener("change", onChange)
  }, [])

  return !!isMobile
}
