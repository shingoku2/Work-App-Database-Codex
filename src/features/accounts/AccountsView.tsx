import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Panel, fieldClass, primaryButtonClass, secondaryButtonClass } from "@/components/ui/Panel";
import type { User, UserRole } from "@/types/db";
import { createUser, listUsers, resetUserPassword, updateUser } from "@/features/connection/connectionApi";

export function AccountsView() {
  const queryClient = useQueryClient();
  const { data = [], error, isLoading } = useQuery({ queryKey: ["users"], queryFn: listUsers });
  const [form, setForm] = useState({ username: "", display_name: "", password: "", role: "user" as UserRole });
  const createMutation = useMutation({
    mutationFn: () => createUser(form),
    onSuccess: async () => {
      setForm({ username: "", display_name: "", password: "", role: "user" });
      await queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Accounts</h2>
        <p className="text-sm text-slate-500">Create users, assign roles, disable access, and reset passwords.</p>
      </div>
      <Panel title="Create Account">
        <form className="grid grid-cols-4 gap-3" onSubmit={(event) => { event.preventDefault(); createMutation.mutate(); }}>
          <input className={fieldClass} required placeholder="Username" value={form.username} onChange={(event) => setForm({ ...form, username: event.target.value })} />
          <input className={fieldClass} required placeholder="Display name" value={form.display_name} onChange={(event) => setForm({ ...form, display_name: event.target.value })} />
          <input className={fieldClass} required minLength={12} type="password" placeholder="Temporary password" value={form.password} onChange={(event) => setForm({ ...form, password: event.target.value })} />
          <select className={fieldClass} value={form.role} onChange={(event) => setForm({ ...form, role: event.target.value as UserRole })}>
            <option value="user">User</option>
            <option value="admin">Admin</option>
          </select>
          <button className={primaryButtonClass} disabled={createMutation.isPending}>Create Account</button>
          {createMutation.error && <span className="col-span-3 text-sm text-red-300">{String(createMutation.error)}</span>}
        </form>
      </Panel>
      <Panel title="Existing Accounts">
        {isLoading ? <div className="text-slate-400">Loading accounts...</div> : (
          <div className="space-y-3">
            {data.map((user) => <UserRow key={user.id} user={user} />)}
          </div>
        )}
        {error && <div className="mt-3 text-sm text-red-300">{String(error)}</div>}
      </Panel>
    </section>
  );
}

function UserRow({ user }: { user: User }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState(user);
  const [password, setPassword] = useState("");
  const updateMutation = useMutation({
    mutationFn: () => updateUser(draft),
    onSuccess: async (updated) => {
      setDraft(updated);
      await queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });
  const passwordMutation = useMutation({
    mutationFn: () => resetUserPassword(user.id, password),
    onSuccess: () => setPassword(""),
  });
  return (
    <div className="grid grid-cols-[1fr_1fr_130px_110px_auto] items-center gap-3 rounded-md border border-white/10 p-3">
      <div>
        <div className="font-medium">{user.username}</div>
        <div className="text-xs text-slate-500">version {user.version}</div>
      </div>
      <input className={fieldClass} value={draft.display_name} onChange={(event) => setDraft({ ...draft, display_name: event.target.value })} />
      <select className={fieldClass} value={draft.role} onChange={(event) => setDraft({ ...draft, role: event.target.value as UserRole })}>
        <option value="user">User</option>
        <option value="admin">Admin</option>
      </select>
      <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} /> Enabled</label>
      <button className={secondaryButtonClass} disabled={updateMutation.isPending} onClick={() => updateMutation.mutate()}>Save</button>
      <div className="col-start-2 col-span-3 flex gap-2">
        <input className={`${fieldClass} flex-1`} type="password" minLength={12} placeholder="New password" value={password} onChange={(event) => setPassword(event.target.value)} />
        <button className={secondaryButtonClass} disabled={password.length < 12 || passwordMutation.isPending} onClick={() => passwordMutation.mutate()}>Reset Password</button>
      </div>
      {(updateMutation.error || passwordMutation.error) && <div className="col-span-5 text-sm text-red-300">{String(updateMutation.error || passwordMutation.error)}</div>}
    </div>
  );
}
