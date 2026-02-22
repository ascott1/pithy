import { saveFile } from "$lib/tauri/fs";

export type SaveState = "idle" | "debouncing" | "saving" | "error";

export class AutoSaveController {
	private path: string | null = null;
	private doc = "";
	private lastSavedDoc = "";
	private debounceMs: number;

	private timer: ReturnType<typeof setTimeout> | null = null;
	private saving = false;
	private generation = 0;
	private flushWaiters: (() => void)[] = [];

	state: SaveState = "idle";
	error: string | null = null;
	dirty = false;
	onState?: (s: SaveState, dirty: boolean, err: string | null) => void;

	constructor(debounceMs = 350) {
		this.debounceMs = debounceMs;
	}

	private setState(s: SaveState, err: string | null = null) {
		this.state = s;
		this.error = err;
		this.dirty = this.doc !== this.lastSavedDoc;
		this.onState?.(s, this.dirty, err);
	}

	setOpenedFile(path: string, contents: string) {
		this.generation++;
		this.cancelPending();
		this.path = path;
		this.doc = contents;
		this.lastSavedDoc = contents;
		this.resolveWaiters();
		this.setState("idle", null);
	}

	setDoc(next: string) {
		this.doc = next;
		if (!this.path) return;

		if (this.doc === this.lastSavedDoc) {
			if (!this.saving && !this.timer) this.setState("idle", null);
			return;
		}
		this.schedule();
	}

	private schedule() {
		if (!this.path) return;
		if (this.timer) clearTimeout(this.timer);
		this.setState("debouncing", null);

		this.timer = setTimeout(() => {
			this.timer = null;
			void this.doSave();
		}, this.debounceMs);
	}

	private cancelPending() {
		if (this.timer) clearTimeout(this.timer);
		this.timer = null;
		if (!this.saving) this.setState("idle", this.error);
	}

	private resolveWaiters() {
		const waiters = this.flushWaiters;
		this.flushWaiters = [];
		for (const resolve of waiters) resolve();
	}

	flushNow() {
		if (!this.path) return;
		if (this.timer) clearTimeout(this.timer);
		this.timer = null;

		if (this.doc === this.lastSavedDoc) {
			if (!this.saving) {
				this.setState("idle", null);
				this.resolveWaiters();
			}
			return;
		}

		if (!this.saving) {
			void this.doSave();
		}
	}

	async flushAndWait() {
		if (this.timer) clearTimeout(this.timer);
		this.timer = null;

		if (this.doc === this.lastSavedDoc && !this.saving) return;

		return new Promise<void>((resolve) => {
			this.flushWaiters.push(resolve);
			if (!this.saving && this.doc !== this.lastSavedDoc) {
				void this.doSave();
			}
		});
	}

	private async doSave() {
		if (this.saving || !this.path) return;
		this.saving = true;

		const gen = this.generation;

		while (this.doc !== this.lastSavedDoc && this.generation === gen) {
			const pathSnapshot = this.path!;
			const docSnapshot = this.doc;

			this.setState("saving", null);

			try {
				await saveFile(pathSnapshot, docSnapshot);

				if (this.generation === gen) {
					this.lastSavedDoc = docSnapshot;
				}
			} catch (e) {
				this.setState("error", String(e));
				break;
			}
		}

		this.saving = false;

		if (this.generation === gen && this.doc === this.lastSavedDoc) {
			this.setState("idle", null);
		}

		this.resolveWaiters();
	}
}
