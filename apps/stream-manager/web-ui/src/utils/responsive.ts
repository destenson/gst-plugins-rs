import { useEffect, useState } from 'react';

export const breakpoints = {
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  '2xl': 1536,
} as const;

export function useBreakpoint() {
  const [breakpoint, setBreakpoint] = useState<keyof typeof breakpoints>('lg');

  useEffect(() => {
    function handleResize() {
      const width = globalThis.innerWidth;
      if (width < breakpoints.sm) setBreakpoint('sm');
      else if (width < breakpoints.md) setBreakpoint('md');
      else if (width < breakpoints.lg) setBreakpoint('lg');
      else if (width < breakpoints.xl) setBreakpoint('xl');
      else setBreakpoint('2xl');
    }

    handleResize();
    globalThis.addEventListener('resize', handleResize);
    return () => globalThis.removeEventListener('resize', handleResize);
  }, []);

  return breakpoint;
}

export function useIsMobile() {
  const breakpoint = useBreakpoint();
  return breakpoint === 'sm' || breakpoint === 'md';
}

export function useIsDesktop() {
  const breakpoint = useBreakpoint();
  return breakpoint === 'lg' || breakpoint === 'xl' || breakpoint === '2xl';
}