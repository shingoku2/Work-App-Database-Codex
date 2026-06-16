import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";
import { fieldClass, primaryButtonClass, secondaryButtonClass } from "@/components/ui/Panel";
import { initialServerUrl } from "@/config/server";
import type { PairingInfo, TunnelKeyInfo, TunnelStatus, User } from "@/types/db";
import {
  generateTunnelKey,
  getConnectionState,
  getTunnelStatus,
  login,
  pairServer,
  probeServer,
  saveTunnelConfig,
  startTunnelConnection,
  unpairServer,
} from "./connectionApi";

export function ConnectionGate({
  children,
}: {
  children: (user: User, serverUrl: string) => ReactNode;
}) {
  const queryClient = useQueryClient();
  const connection = useQuery({
    queryKey: ["connection"],
    queryFn: getConnectionState,
    retry: false,
  });
  const tunnel = useQuery({
    queryKey: ["tunnel"],
    queryFn: getTunnelStatus,
    retry: false,
  });

  if (connection.isLoading || tunnel.isLoading) {
    return <FullPageCard title="Connecting">Loading saved server configuration...</FullPageCard>;
  }
  if (connection.error) {
    return <FullPageCard title="Connection error">{String(connection.error)}</FullPageCard>;
  }
  if (tunnel.error) {
    return <FullPageCard title="Tunnel error">{String(tunnel.error)}</FullPageCard>;
  }

  const tunnelStatus = tunnel.data;
  if (!connection.data?.paired && tunnelStatus?.supported && !tunnelStatus.configured) {
    return <TunnelSetupView status={tunnelStatus} onComplete={() => queryClient.invalidateQueries()} />;
  }
  if (!connection.data?.paired && tunnelStatus?.supported && tunnelStatus.configured && !tunnelStatus.local_port_open) {
    return <TunnelSetupView status={tunnelStatus} onComplete={() => queryClient.invalidateQueries()} />;
  }
  if (connection.data?.status === "repair_required") {
    return (
      <RepairView
        error={connection.data.error}
        onComplete={() => queryClient.invalidateQueries({ queryKey: ["connection"] })}
      />
    );
  }
  if (!connection.data?.paired) {
    return <PairingView initialUrl={tunnelStatus?.local_port_open ? tunnelStatus.local_url : undefined} onComplete={() => queryClient.invalidateQueries({ queryKey: ["connection"] })} />;
  }
  if (!connection.data.user) {
    return (
      <LoginView
        serverUrl={connection.data.url!}
        connectionError={connection.data.error}
        onComplete={() => queryClient.invalidateQueries({ queryKey: ["connection"] })}
      />
    );
  }
  return <>{children(connection.data.user, connection.data.url!)}</>;
}

function TunnelSetupView({ status, onComplete }: { status: TunnelStatus; onComplete: () => void }) {
  const [sshDestination, setSshDestination] = useState("");
  const [sshPort, setSshPort] = useState("22");
  const [identityFile, setIdentityFile] = useState("");
  const [localPort, setLocalPort] = useState("8443");
  const [remoteHost, setRemoteHost] = useState("127.0.0.1");
  const [remotePort, setRemotePort] = useState("8443");
  const [key, setKey] = useState<TunnelKeyInfo | null>(null);
  const generateKey = useMutation({
    mutationFn: generateTunnelKey,
    onSuccess: (created) => {
      setKey(created);
      setIdentityFile(created.identity_file);
    },
  });
  const startExisting = useMutation({ mutationFn: startTunnelConnection, onSuccess: onComplete });
  const save = useMutation({
    mutationFn: () =>
      saveTunnelConfig({
        ssh_destination: sshDestination,
        ssh_port: Number(sshPort),
        identity_file: identityFile || null,
        local_port: Number(localPort),
        remote_host: remoteHost,
        remote_port: Number(remotePort),
      }),
    onSuccess: onComplete,
  });

  return (
    <FullPageCard title="Set up SSH tunnel">
      <div className="space-y-4 text-sm text-slate-300">
        <p>
          Create this computer's own SSH tunnel. Do not use Eddie's SSH login. Generate a local key, have the
          server administrator add the public key to the tunnel account, then save and start the tunnel.
        </p>
        {status.configured && status.error && <ErrorText error={status.error} />}
        {status.configured && (
          <div className="rounded-md border border-amber-400/30 bg-amber-400/10 p-3 text-amber-100">
            A tunnel config exists at {status.config_path}, but the local port is not open.
            <button className={`${secondaryButtonClass} mt-3`} disabled={startExisting.isPending} onClick={() => startExisting.mutate()}>
              Start Existing Tunnel
            </button>
            {startExisting.error && <ErrorText error={startExisting.error} />}
          </div>
        )}
        <button className={secondaryButtonClass} disabled={generateKey.isPending} onClick={() => generateKey.mutate()}>
          Generate This Computer's SSH Key
        </button>
        {generateKey.error && <ErrorText error={generateKey.error} />}
        {key && (
          <div className="rounded-md border border-white/10 bg-black/20 p-4">
            <div className="mb-2 text-xs uppercase text-slate-500">Send this public key to the server administrator</div>
            <textarea className={`${fieldClass} h-28 w-full font-mono text-xs`} readOnly value={key.public_key} />
            <div className="mt-2 text-xs text-slate-400">Private key stays on this computer: {key.identity_file}</div>
          </div>
        )}
        <form
          className="space-y-3"
          onSubmit={(event) => {
            event.preventDefault();
            save.mutate();
          }}
        >
          <input className={`${fieldClass} w-full`} placeholder="SSH destination, e.g. fleet-user@ssh-host.example" value={sshDestination} onChange={(event) => setSshDestination(event.target.value)} />
          <input className={`${fieldClass} w-full`} placeholder="Private key path" value={identityFile} onChange={(event) => setIdentityFile(event.target.value)} />
          <div className="grid grid-cols-3 gap-2">
            <input className={`${fieldClass} w-full`} aria-label="SSH port" value={sshPort} onChange={(event) => setSshPort(event.target.value)} />
            <input className={`${fieldClass} w-full`} aria-label="Local port" value={localPort} onChange={(event) => setLocalPort(event.target.value)} />
            <input className={`${fieldClass} w-full`} aria-label="Remote port" value={remotePort} onChange={(event) => setRemotePort(event.target.value)} />
          </div>
          <input className={`${fieldClass} w-full`} placeholder="Remote host on SSH server" value={remoteHost} onChange={(event) => setRemoteHost(event.target.value)} />
          <button className={primaryButtonClass} disabled={save.isPending || !sshDestination.trim()}>
            Save and Start Tunnel
          </button>
          {save.error && <ErrorText error={save.error} />}
        </form>
      </div>
    </FullPageCard>
  );
}

function PairingView({ initialUrl, onComplete }: { initialUrl?: string; onComplete: () => void }) {
  const [url, setUrl] = useState(initialUrl ?? initialServerUrl);
  const [pairing, setPairing] = useState<PairingInfo | null>(null);
  const probe = useMutation({ mutationFn: probeServer, onSuccess: setPairing });
  const pair = useMutation({
    mutationFn: () => pairServer(url, pairing!),
    onSuccess: onComplete,
  });

  return (
    <FullPageCard title="Connect to Fleet Server">
      <p className="mb-5 text-sm text-slate-400">
        Enter the HTTPS address supplied by your server administrator. This desktop installation stores one server.
      </p>
      {!pairing ? (
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            probe.mutate(url);
          }}
        >
          <input
            className={`${fieldClass} w-full`}
            aria-label="Server URL"
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="https://fleet-server.example:8443"
          />
          <button className={primaryButtonClass} disabled={probe.isPending}>Check Server</button>
          {probe.error && <ErrorText error={probe.error} />}
        </form>
      ) : (
        <div className="space-y-4">
          <div className="rounded-md border border-white/10 bg-black/20 p-4">
            <div className="font-medium">{pairing.server.product} {pairing.server.version}</div>
            <div className="mt-2 text-xs uppercase text-slate-500">Certificate SHA-256 fingerprint</div>
            <code className="mt-1 block break-all text-sm text-sky-200">{pairing.fingerprint_sha256}</code>
          </div>
          <p className="text-sm text-amber-200">
            Confirm this fingerprint with the server administrator before trusting it.
          </p>
          <div className="flex gap-2">
            <button className={primaryButtonClass} disabled={pair.isPending} onClick={() => pair.mutate()}>
              Trust and Connect
            </button>
            <button className={secondaryButtonClass} onClick={() => setPairing(null)}>Back</button>
          </div>
          {pair.error && <ErrorText error={pair.error} />}
        </div>
      )}
    </FullPageCard>
  );
}

function LoginView({
  serverUrl,
  connectionError,
  onComplete,
}: {
  serverUrl: string;
  connectionError: string | null;
  onComplete: () => void;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const mutation = useMutation({
    mutationFn: () => login(username, password),
    onSuccess: onComplete,
  });
  const queryClient = useQueryClient();
  const reset = useMutation({
    mutationFn: unpairServer,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["connection"] }),
  });
  const unavailable = Boolean(connectionError);

  return (
    <FullPageCard title="Sign in">
      <div className="mb-4 text-sm text-slate-400">{serverUrl}</div>
      {unavailable && (
        <div className="mb-4 space-y-3 rounded-md border border-red-400/30 bg-red-400/10 p-3 text-sm text-red-200">
          <div>{connectionError}</div>
          <button className={secondaryButtonClass} disabled={reset.isPending} onClick={() => reset.mutate()}>
            Forget Server and Re-pair
          </button>
        </div>
      )}
      <form
        className="space-y-3"
        onSubmit={(event) => {
          event.preventDefault();
          mutation.mutate();
        }}
      >
        <input className={`${fieldClass} w-full`} autoComplete="username" placeholder="Username" value={username} onChange={(event) => setUsername(event.target.value)} />
        <input className={`${fieldClass} w-full`} autoComplete="current-password" type="password" placeholder="Password" value={password} onChange={(event) => setPassword(event.target.value)} />
        <button className={primaryButtonClass} disabled={mutation.isPending || Boolean(unavailable)}>Sign In</button>
        {mutation.error && <ErrorText error={mutation.error} />}
      </form>
    </FullPageCard>
  );
}

function RepairView({ error, onComplete }: { error: string | null; onComplete: () => void }) {
  const reset = useMutation({ mutationFn: unpairServer, onSuccess: onComplete });
  return (
    <FullPageCard title="Repair server connection">
      <p className="mb-4 text-sm text-red-200">{error ?? "The saved server profile is invalid."}</p>
      <button className={primaryButtonClass} disabled={reset.isPending} onClick={() => reset.mutate()}>
        Reset Saved Server
      </button>
      {reset.error && <ErrorText error={reset.error} />}
    </FullPageCard>
  );
}

function FullPageCard({ title, children }: { title: string; children: ReactNode }) {
  return (
    <main className="grid min-h-screen place-items-center bg-[#101821] p-6 text-slate-100">
      <section className="w-full max-w-xl rounded-xl border border-white/10 bg-[#0b1219] p-8 shadow-2xl">
        <h1 className="mb-3 text-2xl font-semibold">{title}</h1>
        {children}
      </section>
    </main>
  );
}

function ErrorText({ error }: { error: unknown }) {
  return <div className="text-sm text-red-300">{String(error)}</div>;
}
