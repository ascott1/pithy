<script lang="ts">
	import { onMount } from "svelte";
	import {
		updateConfig,
		listThemes,
		type ConfigInfo,
		type ConfigUpdates,
	} from "$lib/tauri/config";

	interface Props {
		configInfo: ConfigInfo;
		onclose: () => void;
		onchange: (newConfig: ConfigInfo) => void;
	}

	let { configInfo, onclose, onchange }: Props = $props();

	// Local state for instant UI feedback
	let vaultDir = $state(configInfo.vaultDirDisplay);
	let autoUpdateLinks = $state(configInfo.autoUpdateLinks);
	let fontSize = $state(configInfo.editor.fontSize);
	let fontFamily = $state(configInfo.editor.fontFamily);
	let lineHeight = $state(configInfo.editor.lineHeight);
	let themeMode = $state(configInfo.theme.mode);
	let themeLight = $state("");
	let themeDark = $state("");
	let dailyDir = $state(configInfo.daily.dir);
	let dailyFormat = $state(configInfo.daily.format);
	let statusBarShow = $state(configInfo.statusBar.show);
	let statusBarShowBacklinks = $state(configInfo.statusBar.showBacklinks);
	let statusBarShowWordCount = $state(configInfo.statusBar.showWordCount);

	let themes = $state<string[]>([]);
	let debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();

	onMount(async () => {
		const themeList = await listThemes();
		themes = themeList;

		// Derive current theme names from CSS content by matching against known themes
		// For now, use config defaults since ConfigInfo sends CSS not names
		// We'll match by checking which theme name produces the current CSS
		themeLight = findThemeName(configInfo.theme.lightCss, "light") ?? "default-light";
		themeDark = findThemeName(configInfo.theme.darkCss, "dark") ?? "default-dark";
	});

	function findThemeName(_css: string, slot: "light" | "dark"): string | null {
		// ConfigInfo doesn't carry theme names, only CSS. We'll read from the TOML
		// by doing a quick update_config round-trip with no changes, but that's wasteful.
		// Instead, since we know the default CSS, match against built-ins.
		// For custom themes, we'll need to rely on the TOML. Let's use a simple heuristic:
		// default-light contains #ffffff, default-dark contains #1c1c1e
		if (slot === "light") {
			if (_css.includes("#ffffff")) return "default-light";
		} else {
			if (_css.includes("#1c1c1e")) return "default-dark";
		}
		// For custom themes, we can't determine the name from CSS alone.
		// Fall back to the default.
		return null;
	}

	async function applyUpdate(updates: ConfigUpdates) {
		try {
			const newConfig = await updateConfig(updates);
			onchange(newConfig);
		} catch (e) {
			console.error("Failed to update config:", e);
		}
	}

	function applyImmediate(updates: ConfigUpdates) {
		void applyUpdate(updates);
	}

	function applyDebounced(key: string, updates: ConfigUpdates, delay = 300) {
		const existing = debounceTimers.get(key);
		if (existing) clearTimeout(existing);
		debounceTimers.set(
			key,
			setTimeout(() => {
				debounceTimers.delete(key);
				void applyUpdate(updates);
			}, delay),
		);
	}

	function handleToggle(field: keyof ConfigUpdates, value: boolean) {
		applyImmediate({ [field]: value });
	}

	function stepFontSize(delta: number) {
		const next = Math.max(8, Math.min(48, fontSize + delta));
		if (next !== fontSize) {
			fontSize = next;
			applyImmediate({ editorFontSize: next });
		}
	}

	function stepLineHeight(delta: number) {
		const next = Math.round(Math.max(1.0, Math.min(3.0, lineHeight + delta)) * 10) / 10;
		if (next !== lineHeight) {
			lineHeight = next;
			applyImmediate({ editorLineHeight: next });
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Escape") {
			e.preventDefault();
			onclose();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="settings-root">
	<div class="settings-header">
		<button class="back-button" onclick={onclose}>
			<svg width="7" height="12" viewBox="0 0 7 12" fill="none">
				<path d="M6 1L1 6L6 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
			Back
		</button>
		<h1 class="settings-title">Settings</h1>
	</div>

	<div class="settings-scroll">
		<!-- General -->
		<section class="settings-section">
			<h2 class="section-label">General</h2>
			<div class="card">
				<div class="row">
					<label class="row-label" for="vault-dir">Vault directory</label>
					<input
						id="vault-dir"
						class="text-input"
						type="text"
						bind:value={vaultDir}
						oninput={() => applyDebounced("vaultDir", { vaultDir })}
					/>
				</div>
				<div class="row">
					<label class="row-label" for="auto-update">Auto-update links</label>
					<button
						id="auto-update"
						class="toggle"
						class:on={autoUpdateLinks}
						onclick={() => { autoUpdateLinks = !autoUpdateLinks; handleToggle("autoUpdateLinks", autoUpdateLinks); }}
						role="switch"
						aria-checked={autoUpdateLinks}
						aria-label="Auto-update links"
					>
						<span class="toggle-knob"></span>
					</button>
				</div>
			</div>
		</section>

		<!-- Editor -->
		<section class="settings-section">
			<h2 class="section-label">Editor</h2>
			<div class="card">
				<div class="row">
					<label class="row-label" id="font-size-label">Font size</label>
					<div class="stepper" role="group" aria-labelledby="font-size-label">
						<button class="stepper-btn" onclick={() => stepFontSize(-1)} disabled={fontSize <= 8} aria-label="Decrease font size">−</button>
						<span class="stepper-value">{fontSize}</span>
						<button class="stepper-btn" onclick={() => stepFontSize(1)} disabled={fontSize >= 48} aria-label="Increase font size">+</button>
					</div>
				</div>
				<div class="row">
					<label class="row-label" for="font-family">Font family</label>
					<input
						id="font-family"
						class="text-input"
						type="text"
						bind:value={fontFamily}
						oninput={() => applyDebounced("fontFamily", { editorFontFamily: fontFamily })}
					/>
				</div>
				<div class="row">
					<label class="row-label" id="line-height-label">Line height</label>
					<div class="stepper" role="group" aria-labelledby="line-height-label">
						<button class="stepper-btn" onclick={() => stepLineHeight(-0.1)} disabled={lineHeight <= 1.0} aria-label="Decrease line height">−</button>
						<span class="stepper-value">{lineHeight.toFixed(1)}</span>
						<button class="stepper-btn" onclick={() => stepLineHeight(0.1)} disabled={lineHeight >= 3.0} aria-label="Increase line height">+</button>
					</div>
				</div>
			</div>
		</section>

		<!-- Appearance -->
		<section class="settings-section">
			<h2 class="section-label">Appearance</h2>
			<div class="card">
				<div class="row">
					<label class="row-label" id="theme-mode-label">Mode</label>
					<div class="segmented" role="radiogroup" aria-labelledby="theme-mode-label">
						{#each ["auto", "light", "dark"] as m}
							<button
								class="seg-btn"
								class:active={themeMode === m}
								onclick={() => { themeMode = m; applyImmediate({ themeMode: m }); }}
							>
								{m.charAt(0).toUpperCase() + m.slice(1)}
							</button>
						{/each}
					</div>
				</div>
				<div class="row">
					<label class="row-label" for="theme-light">Light theme</label>
					<select
						id="theme-light"
						class="select-input"
						bind:value={themeLight}
						onchange={() => applyImmediate({ themeLight })}
					>
						{#each themes as t}
							<option value={t}>{t}</option>
						{/each}
					</select>
				</div>
				<div class="row">
					<label class="row-label" for="theme-dark">Dark theme</label>
					<select
						id="theme-dark"
						class="select-input"
						bind:value={themeDark}
						onchange={() => applyImmediate({ themeDark })}
					>
						{#each themes as t}
							<option value={t}>{t}</option>
						{/each}
					</select>
				</div>
			</div>
		</section>

		<!-- Daily Notes -->
		<section class="settings-section">
			<h2 class="section-label">Daily Notes</h2>
			<div class="card">
				<div class="row">
					<label class="row-label" for="daily-dir">Directory</label>
					<input
						id="daily-dir"
						class="text-input"
						type="text"
						bind:value={dailyDir}
						oninput={() => applyDebounced("dailyDir", { dailyDir })}
					/>
				</div>
				<div class="row">
					<label class="row-label" for="daily-format">Format</label>
					<div class="input-with-hint">
						<input
							id="daily-format"
							class="text-input"
							type="text"
							bind:value={dailyFormat}
							oninput={() => applyDebounced("dailyFormat", { dailyFormat })}
						/>
						<span class="hint">YYYY, MM, DD</span>
					</div>
				</div>
			</div>
		</section>

		<!-- Status Bar -->
		<section class="settings-section">
			<h2 class="section-label">Status Bar</h2>
			<div class="card">
				<div class="row">
					<label class="row-label">Show status bar</label>
					<button
						class="toggle"
						class:on={statusBarShow}
						onclick={() => { statusBarShow = !statusBarShow; handleToggle("statusBarShow", statusBarShow); }}
						role="switch"
						aria-checked={statusBarShow}
						aria-label="Show status bar"
					>
						<span class="toggle-knob"></span>
					</button>
				</div>
				<div class="row">
					<label class="row-label">Show backlinks</label>
					<button
						class="toggle"
						class:on={statusBarShowBacklinks}
						onclick={() => { statusBarShowBacklinks = !statusBarShowBacklinks; handleToggle("statusBarShowBacklinks", statusBarShowBacklinks); }}
						role="switch"
						aria-checked={statusBarShowBacklinks}
						aria-label="Show backlinks"
					>
						<span class="toggle-knob"></span>
					</button>
				</div>
				<div class="row">
					<label class="row-label">Show word count</label>
					<button
						class="toggle"
						class:on={statusBarShowWordCount}
						onclick={() => { statusBarShowWordCount = !statusBarShowWordCount; handleToggle("statusBarShowWordCount", statusBarShowWordCount); }}
						role="switch"
						aria-checked={statusBarShowWordCount}
						aria-label="Show word count"
					>
						<span class="toggle-knob"></span>
					</button>
				</div>
			</div>
		</section>

		<footer class="settings-footer">
			{configInfo.configPath}
		</footer>
	</div>
</div>

<style>
	.settings-root {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: var(--editor-bg);
		color: var(--editor-text);
		font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
		-webkit-font-smoothing: antialiased;
	}

	.settings-header {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 44px 24px 12px;
		flex-shrink: 0;
	}

	.back-button {
		display: flex;
		align-items: center;
		gap: 4px;
		background: none;
		border: none;
		color: var(--accent-color);
		cursor: pointer;
		font-size: 0.8125rem;
		padding: 4px 8px;
		border-radius: 6px;
		transition: background 0.15s;
	}

	.back-button:hover {
		background: color-mix(in srgb, var(--accent-color) 10%, transparent);
	}

	.settings-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
		letter-spacing: -0.01em;
	}

	.settings-scroll {
		flex: 1;
		overflow-y: auto;
		padding: 8px 24px 32px;
		max-width: var(--content-max-width);
		width: 100%;
		margin: 0 auto;
		box-sizing: border-box;
	}

	.settings-section {
		margin-bottom: 24px;
	}

	.section-label {
		font-size: 0.6875rem;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		opacity: 0.45;
		margin: 0 0 6px 2px;
	}

	.card {
		border: 1px solid var(--border-color);
		border-radius: 10px;
		background: color-mix(in srgb, var(--editor-text) 3%, var(--editor-bg));
		overflow: hidden;
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px;
		min-height: 40px;
		gap: 12px;
	}

	.row + .row {
		border-top: 1px solid var(--border-color);
	}

	.row-label {
		font-size: 0.8125rem;
		flex-shrink: 0;
	}

	/* Text inputs */
	.text-input {
		flex: 1;
		min-width: 0;
		max-width: 260px;
		background: color-mix(in srgb, var(--editor-text) 5%, var(--editor-bg));
		border: 1px solid var(--border-color);
		border-radius: 6px;
		padding: 5px 8px;
		font-size: 0.8125rem;
		color: var(--editor-text);
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
		text-align: right;
	}

	.text-input:focus {
		border-color: var(--accent-color);
	}

	/* Select */
	.select-input {
		max-width: 180px;
		background: color-mix(in srgb, var(--editor-text) 5%, var(--editor-bg));
		border: 1px solid var(--border-color);
		border-radius: 6px;
		padding: 5px 8px;
		font-size: 0.8125rem;
		color: var(--editor-text);
		font-family: inherit;
		outline: none;
		cursor: pointer;
		-webkit-appearance: none;
		appearance: none;
		background-image: url("data:image/svg+xml,%3Csvg width='8' height='5' viewBox='0 0 8 5' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L4 4L7 1' stroke='%23888' stroke-width='1.2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
		background-repeat: no-repeat;
		background-position: right 8px center;
		padding-right: 24px;
	}

	.select-input:focus {
		border-color: var(--accent-color);
	}

	/* Toggle switch */
	.toggle {
		position: relative;
		width: 38px;
		height: 22px;
		border-radius: 11px;
		border: none;
		background: color-mix(in srgb, var(--editor-text) 15%, var(--editor-bg));
		cursor: pointer;
		padding: 0;
		flex-shrink: 0;
		transition: background 0.2s;
	}

	.toggle.on {
		background: var(--accent-color);
	}

	.toggle-knob {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: white;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
		transition: transform 0.2s;
	}

	.toggle.on .toggle-knob {
		transform: translateX(16px);
	}

	/* Segmented control */
	.segmented {
		display: flex;
		background: color-mix(in srgb, var(--editor-text) 8%, var(--editor-bg));
		border-radius: 7px;
		padding: 2px;
		gap: 1px;
	}

	.seg-btn {
		flex: 1;
		padding: 4px 14px;
		border: none;
		background: none;
		border-radius: 5px;
		font-size: 0.75rem;
		font-family: inherit;
		color: var(--editor-text);
		cursor: pointer;
		opacity: 0.6;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.seg-btn.active {
		background: color-mix(in srgb, var(--editor-text) 6%, var(--editor-bg));
		opacity: 1;
		box-shadow: 0 1px 3px var(--shadow-color);
	}

	/* Stepper */
	.stepper {
		display: flex;
		align-items: center;
		gap: 0;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		overflow: hidden;
		background: color-mix(in srgb, var(--editor-text) 5%, var(--editor-bg));
	}

	.stepper-btn {
		width: 30px;
		height: 28px;
		border: none;
		background: none;
		color: var(--accent-color);
		cursor: pointer;
		font-size: 1rem;
		font-family: inherit;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 0.1s;
	}

	.stepper-btn:hover:not(:disabled) {
		background: color-mix(in srgb, var(--accent-color) 10%, transparent);
	}

	.stepper-btn:disabled {
		opacity: 0.25;
		cursor: default;
	}

	.stepper-value {
		min-width: 36px;
		text-align: center;
		font-size: 0.8125rem;
		font-variant-numeric: tabular-nums;
		border-left: 1px solid var(--border-color);
		border-right: 1px solid var(--border-color);
		padding: 0 4px;
	}

	/* Input with hint */
	.input-with-hint {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		justify-content: flex-end;
	}

	.hint {
		font-size: 0.6875rem;
		opacity: 0.4;
		white-space: nowrap;
	}

	.input-with-hint .text-input {
		max-width: 160px;
	}

	/* Footer */
	.settings-footer {
		padding: 16px 0 0;
		font-size: 0.6875rem;
		opacity: 0.3;
		text-align: center;
		user-select: text;
		word-break: break-all;
	}
</style>
