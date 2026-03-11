// types.ts
export type TaskStatus =
	| "todo"
	| "done"
	| "in_progress"
	| "waiting"
	| "cancelled";

export interface Task {
	text: string;
	status: TaskStatus;
	due: string | null; // "YYYY-MM-DD"
	done_date: string | null;
	start: string | null;
	source_file: string;
	line: number;
}

export interface ProjectProgress {
	name: string;
	file: string;
	completed: number;
	total: number;
	percent: number;
}

export interface VaultSummary {
	inbox_count: number;
	due_today: Task[];
	overdue: Task[];
	projects: ProjectProgress[];
}

export type ListItemKind = { task: TaskStatus } | "bullet";

export interface ListItem {
	text: string;
	kind: ListItemKind;
	indent: number;
	due: string | null;
	done_date: string | null;
	start: string | null;
	source_file: string;
	line: number;
}

export interface ProjectTasks {
	name: string;
	file: string;
	items: ListItem[];
}

export interface WeeklyTasks {
	week_start: string; // "YYYY-MM-DD"
	week_end: string;
	projects: ProjectTasks[];
}

export type ViewId = "summary" | "weekly";

export type NoteKind = "daily" | "weekly";

export interface CreateNoteResponse {
	path: string;
	created: boolean;
}
