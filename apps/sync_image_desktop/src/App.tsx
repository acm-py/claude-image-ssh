import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type ConfigDraft = {
  user_name: string;
  hotkey: string;
  upload: {
    host: string;
    port: number;
    user: string;
    auth_method: "key" | "password";
    private_key_path: string;
    password: string;
    shared_image_root: string;
  };
};

type LoadConfigResponse = {
  config: ConfigDraft;
  path: string;
  exists: boolean;
};

type SaveConfigResponse = {
  path: string;
};

type RuntimeStatusResponse = {
  status: "running" | "stopped";
};

type InteractionRequest =
  | {
      kind: "trust_host_key";
      host: string;
      port: number;
      fingerprint: string;
    }
  | {
      kind: "private_key_passphrase";
      private_key_path: string;
    }
  | {
      kind: "password";
      host: string;
      port: number;
      user: string;
    };

type CommandResponse =
  | {
      status: "ok";
      message: string;
      runtime_status?: "running" | "stopped";
    }
  | {
      status: "needs_interaction";
      request: InteractionRequest;
    };

type InteractionReply =
  | {
      kind: "trust_host_key";
      trusted: boolean;
    }
  | {
      kind: "private_key_passphrase";
      passphrase: string;
    }
  | {
      kind: "password";
      password: string;
    };

type PendingCommand = "check_connection" | "start_runtime" | null;

const defaultConfig = (): ConfigDraft => ({
  user_name: "",
  hotkey: "Ctrl+Alt+U",
  upload: {
    host: "",
    port: 22,
    user: "",
    auth_method: "key",
    private_key_path: "",
    password: "",
    shared_image_root: "",
  },
});

export default function App() {
  const [config, setConfig] = useState<ConfigDraft>(defaultConfig);
  const [loadedPath, setLoadedPath] = useState("");
  const [message, setMessage] = useState("Loading configuration...");
  const [runtimeStatus, setRuntimeStatus] = useState<"running" | "stopped">(
    "stopped",
  );
  const [isBusy, setIsBusy] = useState(false);
  const [pendingCommand, setPendingCommand] = useState<PendingCommand>(null);
  const [interaction, setInteraction] = useState<InteractionRequest | null>(null);
  const [passphrase, setPassphrase] = useState("");

  const isRunning = runtimeStatus === "running";

  useEffect(() => {
    void initialize();
  }, []);

  const configPathLabel = useMemo(() => {
    if (!loadedPath) {
      return "Default config path";
    }
    return loadedPath;
  }, [loadedPath]);

  async function initialize() {
    try {
      const [loaded, status] = await Promise.all([
        invoke<LoadConfigResponse>("load_config"),
        invoke<RuntimeStatusResponse>("runtime_status"),
      ]);
      setConfig(loaded.config);
      setLoadedPath(loaded.path);
      setRuntimeStatus(status.status);
      setMessage(
        loaded.exists
          ? "Configuration loaded."
          : "No saved config yet. Fill the fields and save.",
      );
    } catch (error) {
      setMessage(toErrorMessage(error));
    }
  }

  function setField<K extends keyof ConfigDraft>(key: K, value: ConfigDraft[K]) {
    setConfig((current) => ({ ...current, [key]: value }));
  }

  function setUploadField<K extends keyof ConfigDraft["upload"]>(
    key: K,
    value: ConfigDraft["upload"][K],
  ) {
    setConfig((current) => ({
      ...current,
      upload: { ...current.upload, [key]: value },
    }));
  }

  async function handleBrowseKey() {
    const selected = await open({
      multiple: false,
      directory: false,
    });

    if (typeof selected === "string") {
      setUploadField("private_key_path", selected);
    }
  }

  async function handleSave() {
    setIsBusy(true);
    try {
      const response = await invoke<SaveConfigResponse>("save_config", { config });
      setLoadedPath(response.path);
      setMessage(`Configuration saved to ${response.path}`);
    } catch (error) {
      setMessage(toErrorMessage(error));
    } finally {
      setIsBusy(false);
    }
  }

  async function handleCheck() {
    await runCommand("check_connection");
  }

  async function handleStart() {
    await runCommand("start_runtime");
  }

  async function handleStop() {
    setIsBusy(true);
    try {
      const response = await invoke<RuntimeStatusResponse>("stop_runtime");
      setRuntimeStatus(response.status);
      setMessage("Upload runtime stopped.");
    } catch (error) {
      setMessage(toErrorMessage(error));
    } finally {
      setIsBusy(false);
    }
  }

  async function runCommand(command: Exclude<PendingCommand, null>, reply?: InteractionReply) {
    setIsBusy(true);
    setPendingCommand(command);
    try {
      const response = await invoke<CommandResponse>(command, {
        config,
        interaction: reply ?? null,
      });

      if (response.status === "needs_interaction") {
        setInteraction(response.request);
        setPassphrase("");
        setMessage("Additional confirmation required.");
        return;
      }

      setInteraction(null);
      setPendingCommand(null);
      setMessage(response.message);
      if (response.runtime_status) {
        setRuntimeStatus(response.runtime_status);
      }
    } catch (error) {
      setInteraction(null);
      setPendingCommand(null);
      setMessage(toErrorMessage(error));
    } finally {
      setIsBusy(false);
    }
  }

  async function submitInteraction(reply: InteractionReply) {
    if (!pendingCommand) {
      setInteraction(null);
      return;
    }
    await runCommand(pendingCommand, reply);
  }

  return (
    <main className="app-shell">
      <section className="app-header">
        <div>
          <p className="eyebrow">Sync Image</p>
          <h1>Desktop Control Panel</h1>
          <p className="subtle">{configPathLabel}</p>
        </div>
        <div className={`status-pill ${isRunning ? "running" : "stopped"}`}>
          {isRunning ? "Running" : "Stopped"}
        </div>
      </section>

      <section className="panel">
        <header className="panel-header">
          <div>
            <h2>Client Configuration</h2>
            <p>Edit the same fields used by the existing Windows client.</p>
          </div>
          <button
            className="ghost-button"
            type="button"
            onClick={() => setConfig(defaultConfig())}
            disabled={isBusy}
          >
            Reset
          </button>
        </header>

        <div className="form-grid">
          <Field label="User name">
            <input
              value={config.user_name}
              onChange={(event) => setField("user_name", event.target.value)}
            />
          </Field>
          <Field label="Hotkey">
            <input
              value={config.hotkey}
              onChange={(event) => setField("hotkey", event.target.value)}
            />
          </Field>
          <Field label="Upload host">
            <input
              value={config.upload.host}
              onChange={(event) => setUploadField("host", event.target.value)}
            />
          </Field>
          <Field label="Upload port">
            <input
              type="number"
              min={1}
              max={65535}
              value={config.upload.port}
              onChange={(event) =>
                setUploadField("port", Number.parseInt(event.target.value || "22", 10))
              }
            />
          </Field>
          <Field label="Upload user">
            <input
              value={config.upload.user}
              onChange={(event) => setUploadField("user", event.target.value)}
            />
          </Field>
          <Field label="Shared image root">
            <input
              value={config.upload.shared_image_root}
              onChange={(event) =>
                setUploadField("shared_image_root", event.target.value)
              }
            />
          </Field>
        </div>

        <Field label="Authentication method">
          <select
            value={config.upload.auth_method}
            onChange={(event) =>
              setUploadField(
                "auth_method",
                event.target.value as ConfigDraft["upload"]["auth_method"],
              )
            }
          >
            <option value="key">Private key</option>
            <option value="password">Password</option>
          </select>
        </Field>

        {config.upload.auth_method === "key" ? (
          <Field label="Private key path">
            <div className="file-row">
              <input
                value={config.upload.private_key_path}
                onChange={(event) =>
                  setUploadField("private_key_path", event.target.value)
                }
              />
              <button
                className="ghost-button"
                type="button"
                onClick={handleBrowseKey}
                disabled={isBusy}
              >
                Browse
              </button>
            </div>
          </Field>
        ) : (
          <Field label="Password (stored in plain text)">
            <input
              type="password"
              value={config.upload.password}
              placeholder="Leave empty to enter on first connection"
              onChange={(event) => setUploadField("password", event.target.value)}
            />
          </Field>
        )}

        <div className="button-row">
          <button type="button" onClick={handleSave} disabled={isBusy}>
            Save Config
          </button>
          <button
            className="ghost-button"
            type="button"
            onClick={handleCheck}
            disabled={isBusy}
          >
            Run Check
          </button>
        </div>
      </section>

      <section className="panel">
        <header className="panel-header">
          <div>
            <h2>Runtime Control</h2>
            <p>Start or stop the global hotkey upload runtime.</p>
          </div>
        </header>

        <div className="button-row">
          <button type="button" onClick={handleStart} disabled={isBusy || isRunning}>
            Start Upload Runtime
          </button>
          <button
            className="ghost-button"
            type="button"
            onClick={handleStop}
            disabled={isBusy || !isRunning}
          >
            Stop Runtime
          </button>
        </div>
      </section>

      <section className="message-bar">{message}</section>

      {interaction && (
        <div className="modal-backdrop">
          <div className="modal">
            {interaction.kind === "trust_host_key" ? (
              <>
                <h3>Trust SSH Host Key</h3>
                <p>
                  {interaction.host}:{interaction.port}
                </p>
                <pre>{interaction.fingerprint}</pre>
                <div className="button-row">
                  <button
                    type="button"
                    onClick={() =>
                      submitInteraction({
                        kind: "trust_host_key",
                        trusted: true,
                      })
                    }
                  >
                    Trust
                  </button>
                  <button
                    className="ghost-button"
                    type="button"
                    onClick={() =>
                      submitInteraction({
                        kind: "trust_host_key",
                        trusted: false,
                      })
                    }
                  >
                    Cancel
                  </button>
                </div>
              </>
            ) : interaction.kind === "private_key_passphrase" ? (
              <>
                <h3>Private Key Passphrase</h3>
                <p>{interaction.private_key_path}</p>
                <input
                  type="password"
                  value={passphrase}
                  onChange={(event) => setPassphrase(event.target.value)}
                  placeholder="Enter passphrase"
                />
                <div className="button-row">
                  <button
                    type="button"
                    onClick={() =>
                      submitInteraction({
                        kind: "private_key_passphrase",
                        passphrase,
                      })
                    }
                  >
                    Continue
                  </button>
                  <button
                    className="ghost-button"
                    type="button"
                    onClick={() => setInteraction(null)}
                  >
                    Cancel
                  </button>
                </div>
              </>
            ) : (
              <>
                <h3>SSH Password</h3>
                <p>
                  {interaction.user}@{interaction.host}:{interaction.port}
                </p>
                <p className="subtle">
                  Saved to the config file in plain text after a successful
                  connection.
                </p>
                <input
                  type="password"
                  value={passphrase}
                  onChange={(event) => setPassphrase(event.target.value)}
                  placeholder="Enter password"
                />
                <div className="button-row">
                  <button
                    type="button"
                    onClick={() =>
                      submitInteraction({
                        kind: "password",
                        password: passphrase,
                      })
                    }
                  >
                    Continue
                  </button>
                  <button
                    className="ghost-button"
                    type="button"
                    onClick={() => setInteraction(null)}
                  >
                    Cancel
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </main>
  );
}

function Field(props: { label: string; children: React.ReactNode }) {
  return (
    <label className="field">
      <span>{props.label}</span>
      {props.children}
    </label>
  );
}

function toErrorMessage(error: unknown) {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object" && "message" in error) {
    return String(error.message);
  }
  return "Operation failed.";
}
