'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function WorkAtelierPage() {
  const router = useRouter();

  useEffect(() => {
    router.replace('/work/plans');
  }, [router]);

  return (
    <div className="flex flex-col items-center justify-center h-full gap-2">
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
        Redirecting to Plans…
      </span>
    </div>
  );
}
