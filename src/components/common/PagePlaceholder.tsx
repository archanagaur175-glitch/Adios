import type { ReactNode } from "react";

type Props = {
  title: string;
  children?: ReactNode;
};

export function PagePlaceholder({ title, children }: Props) {
  return (
    <section className="page">
      <h1 className="text-28 font-display">{title}</h1>
      <p className="text-ink-3">{children ?? "Coming in a later step."}</p>
    </section>
  );
}
