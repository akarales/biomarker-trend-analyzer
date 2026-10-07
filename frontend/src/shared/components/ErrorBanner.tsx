interface Props {
  message: string | null;
}

/** Inline error banner; renders nothing without a message. */
export function ErrorBanner({ message }: Props) {
  if (!message) return null;
  return (
    <p role="alert" className="rounded-md border border-destructive/50 bg-destructive/10 p-2 text-xs text-destructive">
      {message}
    </p>
  );
}
