import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Panel, fieldClass, primaryButtonClass, secondaryButtonClass, codeClass } from "@/components/ui/Panel";
import type { User } from "@/types/db";
import { changePassword, logout, unpairServer } from "@/features/connection/connectionApi";

export function SettingsView({ user, serverUrl }: { user: User; serverUrl: string }) {
  const queryClient = useQueryClient();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [backupFormat, setBackupFormat] = useState("custom");
  const [backupCompress, setBackupCompress] = useState(true);
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
          <button
            className={secondaryButtonClass}
            disabled={unpairMutation.isPending}
            onClick={() =>
              window.confirm("Forget this server and remove the saved session?") &&
              unpairMutation.mutate()
            }
          >
            Forget Server
          </button>
        </div>
      </Panel>

      <Panel title="Change Password">
        <form
          className="flex flex-wrap gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            passwordMutation.mutate();
          }}
        >
          <input
            className={fieldClass}
            required
            type="password"
            placeholder="Current password"
            value={currentPassword}
            onChange={(event) => setCurrentPassword(event.target.value)}
          />
          <input
            className={fieldClass}
            required
            minLength={12}
            type="password"
            placeholder="New password"
            value={newPassword}
            onChange={(event) => setNewPassword(event.target.value)}
          />
          <button className={primaryButtonClass} disabled={passwordMutation.isPending}>
            Change Password
          </button>
        </form>
        {passwordMutation.error && (
          <div className="mt-3 text-sm text-red-300">{String(passwordMutation.error)}</div>
        )}
      </Panel>

      {user.role === "admin" && (
        <Panel title="Database Backup / Restore (Server CLI)">
          <p className="text-sm text-slate-400 mb-4">
            These commands must be run on the server host where the Fleet Server is deployed.
          </p>
          <div className="space-y-4">
            <div>
              <h4 className="font-medium mb-2">Create Backup</h4>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-3">
                <select
                  className={fieldClass}
                  value={backupFormat}
                  onChange={(e) => setBackupFormat(e.target.value)}
                >
                  <option value="plain">Plain SQL (.sql)</option>
                  <option value="custom">Custom (compressed)</option>
                  <option value="directory">Directory format</option>
                  <option value="tar">Tar archive</option>
                </select>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={backupCompress}
                    onChange={(e) => setBackupCompress(e.target.checked)}
                  />
                  Compress (custom/dir/tar)
                </label>
              </div>
              <code className={codeClass}>
                antminer-fleet-server --config /etc/antminer-fleet/server.toml backup \
                --output backup.sql --format {backupFormat} {backupCompress && '--compress'}
              </code>
            </div>
            <div className="border-t border-white/10 pt-4">
              <h4 className="font-medium mb-2">Restore Backup</h4>
              <code className={codeClass}>
                antminer-fleet-server --config /etc/antminer-fleet/server.toml restore backup.sql \
                --clean --no-owner
              </code>
              <p className="text-xs text-slate-500 mt-2">
                Use --clean to drop existing objects before restore. Use --no-owner to avoid ownership issues.
              </p>
            </div>
          </div>
        </Panel>
      )}

      <button
        className={secondaryButtonClass}
        disabled={logoutMutation.isPending}
        onClick={() => logoutMutation.mutate()}
      >
        Sign Out
      </button>
    </section>
  );
}
