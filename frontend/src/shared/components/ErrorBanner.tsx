interface Props {
  message: string | null;
}

/** Inline error banner; renders nothing without a message. */
export function ErrorBanner({ message }: Props) {
  if (!message) return null;
  return (
    <p className="rounded border border-red-300 bg-red-50 p-2 text-xs text-red-800">
      {message}
    </p>
  );
}
