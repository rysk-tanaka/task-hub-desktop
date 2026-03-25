import type { ListItem, ListItemKind, TaskStatus, WeeklyTasks } from "../types";

const STATUS_MARKER: Record<TaskStatus, string> = {
	todo: "\u23f9\ufe0f",
	done: "\u2705",
	in_progress: "\u25b6\ufe0f",
	waiting: "\u23f8\ufe0f",
	cancelled: "\u274c",
};

function getTaskStatus(kind: ListItemKind): TaskStatus | null {
	if (typeof kind === "object" && "task" in kind) {
		return kind.task;
	}
	return null;
}

function formatRange(start: string, end: string): string {
	const s = new Date(`${start}T00:00:00`);
	const e = new Date(`${end}T00:00:00`);
	return `${s.getMonth() + 1}/${s.getDate()} ~ ${e.getMonth() + 1}/${e.getDate()}`;
}

interface TaskGroup {
	parent: ListItem;
	children: ListItem[];
}

function groupItems(items: ListItem[]): TaskGroup[] {
	const groups: TaskGroup[] = [];
	let i = 0;
	while (i < items.length) {
		const parent = items[i];
		const children: ListItem[] = [];
		i += 1;
		while (i < items.length && items[i].indent > parent.indent) {
			children.push(items[i]);
			i += 1;
		}
		groups.push({ parent, children });
	}
	return groups;
}

const INDENT_UNIT = 4;

function ListItemRow({
	item,
	isParent,
	hasChildren = false,
	baseIndent = 0,
}: {
	item: ListItem;
	isParent: boolean;
	hasChildren?: boolean;
	baseIndent?: number;
}) {
	const status = getTaskStatus(item.kind);
	const isBullet = item.kind === "bullet";
	const isDimmed = isBullet || status === "done" || status === "cancelled";

	return (
		<div
			style={{
				display: "flex",
				alignItems: "baseline",
				gap: 8,
				paddingLeft:
					8 +
					(isParent
						? 0
						: Math.max(
								0,
								Math.floor((item.indent - baseIndent) / INDENT_UNIT) - 1,
							) * 16),
				paddingTop: isParent ? 6 : 3,
				paddingBottom: isParent ? 4 : 3,
				fontSize: "var(--font-base)",
			}}
		>
			<span style={{ width: 20, textAlign: "center", flexShrink: 0 }}>
				{status ? STATUS_MARKER[status] : "\u2022"}
			</span>
			<span
				style={{
					flex: 1,
					color: isDimmed ? "var(--text-muted)" : "var(--text)",
					fontWeight: isParent && hasChildren ? 500 : "normal",
					textDecoration:
						status === "done" || status === "cancelled"
							? "line-through"
							: "none",
				}}
			>
				{item.text}
			</span>
			{item.start && (
				<span
					style={{
						fontSize: "var(--font-base)",
						color: "var(--text-muted)",
						flexShrink: 0,
					}}
				>
					{item.start}
				</span>
			)}
		</div>
	);
}

function TaskGroupBlock({ group }: { group: TaskGroup }) {
	return (
		<div style={styles.taskGroup}>
			<ListItemRow
				item={group.parent}
				isParent={true}
				hasChildren={group.children.length > 0}
			/>
			{group.children.length > 0 && (
				<div style={styles.childrenBlock}>
					{group.children.map((child) => (
						<ListItemRow
							key={`${child.source_file}:${child.line}`}
							item={child}
							isParent={false}
							baseIndent={group.parent.indent}
						/>
					))}
				</div>
			)}
		</div>
	);
}

export function WeeklySection({
	weeklyTasks,
	weekOffset,
	onChangeWeek,
}: {
	weeklyTasks: WeeklyTasks;
	weekOffset: number;
	onChangeWeek: (offset: number) => void;
}) {
	const weekLabel =
		weekOffset === 0
			? "今週"
			: weekOffset === -1
				? "先週"
				: weekOffset === 1
					? "来週"
					: `${weekOffset > 0 ? "+" : ""}${weekOffset}w`;

	return (
		<section style={{ marginBottom: 20 }}>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					gap: 12,
					marginBottom: 12,
				}}
			>
				<h3 style={styles.sectionTitle}>{weekLabel}の予定</h3>
				<div style={{ display: "flex", alignItems: "center", gap: 4 }}>
					<button
						type="button"
						aria-label="前の週"
						style={styles.navButton}
						onClick={() => onChangeWeek(weekOffset - 1)}
					>
						&#9664;
					</button>
					<span
						style={{
							fontSize: "var(--font-sm)",
							color: "var(--text-muted)",
							minWidth: 40,
							textAlign: "center",
						}}
					>
						{weekLabel}
					</span>
					<button
						type="button"
						aria-label="次の週"
						style={styles.navButton}
						onClick={() => onChangeWeek(weekOffset + 1)}
					>
						&#9654;
					</button>
				</div>
				<span
					style={{ fontSize: "var(--font-sm)", color: "var(--text-muted)" }}
				>
					{formatRange(weeklyTasks.week_start, weeklyTasks.week_end)}
				</span>
			</div>

			{weeklyTasks.projects.length === 0 ? (
				<div
					style={{
						color: "var(--text-muted)",
						fontSize: "var(--font-base)",
						padding: "12px 0",
					}}
				>
					タスクはありません
				</div>
			) : (
				weeklyTasks.projects.map((project) => (
					<div key={project.file} style={{ marginBottom: 16 }}>
						<h4 style={styles.projectName}>{project.name}</h4>
						{groupItems(project.items).map((group) => (
							<TaskGroupBlock
								key={`${group.parent.source_file}:${group.parent.line}`}
								group={group}
							/>
						))}
					</div>
				))
			)}
		</section>
	);
}

const styles: Record<string, React.CSSProperties> = {
	sectionTitle: {
		fontSize: "var(--font-sm)",
		fontWeight: 600,
		color: "var(--text-muted)",
		textTransform: "uppercase",
		letterSpacing: "0.06em",
		margin: 0,
	},
	navButton: {
		background: "transparent",
		border: "1px solid var(--border)",
		borderRadius: 4,
		color: "var(--text-muted)",
		cursor: "pointer",
		fontSize: "var(--font-sm)",
		padding: "2px 6px",
		lineHeight: 1,
	},
	projectName: {
		fontSize: "var(--font-base)",
		fontWeight: 600,
		color: "var(--accent)",
		margin: "0 0 6px 0",
	},
	taskGroup: {
		borderBottom: "1px solid var(--border)",
		padding: "4px 0",
	},
	childrenBlock: {
		borderLeft: "2px solid var(--border)",
		marginLeft: 18,
		paddingLeft: 8,
	},
};
