import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Panel, fieldClass, primaryButtonClass, secondaryButtonClass } from "@/components/ui/Panel";
import type { User } from "@/types/db";
import { changePassword, logout, unpairServer } from "@/features/connection/connectionApi";

export function SettingsView({ user, serverUrl }: { user: User; serverUrl: string }) {
  const queryClient = useQueryClient();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const refresh = async () => queryClient.invalidateQueries({ queryKey: ["connection"] });
  const passwordMutation = useMutation({
    mutationFn: () => changePassword(currentPassword, newPassword),
    onSuccess: refresh,
  });
  const logoutMutation = useMutation({ mutationFn: logout, onSuccess: refresh });
  const unpairMutation = useMutation({ mutationFn: unpairServer, onSuccess: refresh });

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Settings</h2>
        <p className="text-sm text-slate-500">Signed in as {user.display_name} ({user.username}).</p>
      </div>
      <Panel title="Server">
        <div className="flex items-center justify-between gap-4">
          <code className="text-sky-200">{serverUrl}</code>
          <button className={secondaryButtonClass} disabled={unpairMutation.isPending} onClick={() => window.confirm("Forget this server and remove the saved session?") && unpairMutation.mutate()}>Forget Server</button>
        </div>
      </Panel>
      <Panel title="Change Password">
        <form className="flex flex-wrap gap-3" onSubmit={(event) => { event.preventDefault(); passwordMutation.mutate(); }}>
          <input className={fieldClass} required type="password" placeholder="Current password" value={currentPassword} onChange={(event) => setCurrentPassword(event.target.value)} />
          <input className={fieldClass} required minLength={12} type="password" placeholder="New password" value={newPassword} onChange={(event) => setNewPassword(event.target.value)} />
          <button className={primaryButtonClass} disabled={passwordMutation.isPending}>Change Password</button>
        </form>
        {passwordMutation.error && <div className="mt-3 text-sm text-red-300">{String(passwordMutation.error)}</div>}
      </Panel>
      <button className={secondaryButtonClass} disabled={logoutMutation.isPending} onClick={() => logoutMutation.mutate()}>Sign Out</button>
    </section>
  );
}
