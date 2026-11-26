import { useEffect, useRef } from "react";

export type Confirmation = { title: string; message: string; confirm: string; action: () => void | Promise<void> };

/** A modal that asks before a destructive action. Escape or Cancel closes it. */
export function ConfirmDialog({ request, close }: { request: Confirmation; close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => { if (element?.open) element.close(); };
  }, []);
  return <dialog ref={dialog} className="confirm-dialog" aria-labelledby="confirm-title" aria-describedby="confirm-message" onCancel={event => { event.preventDefault(); close(); }}>
    <h2 id="confirm-title">{request.title}</h2>
    <p id="confirm-message">{request.message}</p>
    <div className="confirm-actions"><button autoFocus className="secondary" onClick={close}>Cancel</button><button className="danger-button" onClick={() => { close(); void request.action(); }}>{request.confirm}</button></div>
  </dialog>;
}
