import { useState } from "react";
import { native, type CustomApp } from "./native";
import { Icon } from "./Icon";

export function CustomAppEditor({
  initial,
  onSaved,
  onCancel,
}: {
  initial: CustomApp | null;
  onSaved: (id: string | null) => Promise<void>;
  onCancel: () => void;
}) {
  const [profile, setProfile] = useState<CustomApp>(
    () =>
      initial ?? {
        executable: "",
        name: "",
        shortcuts: [{ id: "1", title: "", keys: "" }],
      },
  );
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [confirmRemove, setConfirmRemove] = useState(false);
  async function perform(action: () => Promise<void>) {
    setPending(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setPending(false);
    }
  }
  return (
    <section
      className="cz-app-editor"
      aria-label={initial ? "Edit custom app" : "Add custom app"}
    >
      <header>
        <h2>{initial ? "Edit app" : "Add app"}</h2>
        <button disabled={pending} onClick={onCancel}>
          Cancel
        </button>
      </header>
      <p>
        Choose the app you want to use with PieAll, then add its keyboard
        shortcuts. This configures a ring; it never launches the executable.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void perform(async () => {
            const id = await native.saveCustomApp(
              profile,
              initial?.executable ?? null,
            );
            await onSaved(id);
          });
        }}
      >
        <fieldset disabled={pending}>
          <legend className="sr-only">App and shortcuts</legend>
          <div className="cz-app-executable">
            <span>Executable</span>
            <strong>{profile.executable || "No app selected"}</strong>
            <button
              type="button"
              onClick={() =>
                void perform(async () => {
                  const executable = await native.pickCustomApp();
                  if (executable)
                    setProfile((p) => ({
                      ...p,
                      executable: executable[0],
                      executable_path: executable[1],
                      name: p.name || executable[0].slice(0, -4),
                    }));
                })
              }
            >
              <Icon name="files" size={16} />
              {profile.executable ? "Change executable" : "Choose executable"}
            </button>
          </div>
          <p className="cz-hint">
            Matches the executable name, regardless of installation folder. Apps
            with bundled profiles already appear in the app list.
          </p>
          <label className="cz-app-name">
            App name
            <input
              autoFocus
              required
              maxLength={80}
              value={profile.name}
              onChange={(e) => setProfile({ ...profile, name: e.target.value })}
            />
          </label>
          <div className="cz-app-shortcut-heading">
            <h3>Keyboard shortcuts</h3>
            <span>{profile.shortcuts.length} / 32</span>
          </div>
          <p className="cz-hint" id="custom-shortcut-help">
            Use keys such as Ctrl+S, Ctrl+Shift+P or Alt+Left. For a sequence,
            separate up to three chords with spaces. The first six shortcuts
            become default slices; all are available in Add action.
          </p>
          <ol className="cz-app-shortcuts">
            {profile.shortcuts.map((binding, index) => (
              <li key={binding.id}>
                <label>
                  Action {index + 1}
                  <input
                    required
                    maxLength={80}
                    value={binding.title}
                    placeholder="Save"
                    onChange={(e) =>
                      setProfile({
                        ...profile,
                        shortcuts: profile.shortcuts.map((b) =>
                          b.id === binding.id
                            ? { ...b, title: e.target.value }
                            : b,
                        ),
                      })
                    }
                  />
                </label>
                <label>
                  Keys
                  <input
                    required
                    maxLength={120}
                    aria-label={`Keys for action ${index + 1}`}
                    aria-describedby="custom-shortcut-help"
                    value={binding.keys}
                    placeholder="Ctrl+S"
                    onChange={(e) =>
                      setProfile({
                        ...profile,
                        shortcuts: profile.shortcuts.map((b) =>
                          b.id === binding.id
                            ? { ...b, keys: e.target.value }
                            : b,
                        ),
                      })
                    }
                  />
                </label>
                <button
                  type="button"
                  className="icon-button"
                  disabled={profile.shortcuts.length === 1}
                  aria-label={`Remove shortcut ${index + 1}`}
                  onClick={() =>
                    setProfile({
                      ...profile,
                      shortcuts: profile.shortcuts.filter(
                        (b) => b.id !== binding.id,
                      ),
                    })
                  }
                >
                  <Icon name="close" size={16} />
                </button>
              </li>
            ))}
          </ol>
          <button
            type="button"
            disabled={profile.shortcuts.length >= 32}
            onClick={() =>
              setProfile({
                ...profile,
                shortcuts: [
                  ...profile.shortcuts,
                  { id: crypto.randomUUID(), title: "", keys: "" },
                ],
              })
            }
          >
            <Icon name="plus" size={16} />
            Add shortcut
          </button>
          <div className="cz-app-save">
            <button type="submit" disabled={!profile.executable}>
              {pending ? "Saving…" : "Save app"}
            </button>
            {initial && (
              <button type="button" onClick={() => setConfirmRemove(true)}>
                Remove app
              </button>
            )}
          </div>
          {confirmRemove && (
            <div
              className="cz-app-remove"
              role="group"
              aria-label="Confirm app removal"
            >
              <p>
                Remove {initial?.name} and its shortcuts? This does not
                uninstall the app.
              </p>
              <button
                type="button"
                onClick={() =>
                  void perform(async () => {
                    await native.removeCustomApp(initial!.executable);
                    await onSaved(null);
                  })
                }
              >
                Remove profile
              </button>
              <button type="button" onClick={() => setConfirmRemove(false)}>
                Keep app
              </button>
            </div>
          )}
        </fieldset>
      </form>
      {error && (
        <p className="cz-note" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
