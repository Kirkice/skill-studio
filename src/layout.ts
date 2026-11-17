import { designs, type Role } from './nodeDesign.ts';
export type Layout = Record<string, {x:number; y:number}>;
export type LayoutFile = { version: 1; positions: Layout; roles: Record<string, Role> };
export function parseLayout(text: string): LayoutFile {
 const value: unknown = JSON.parse(text);
 if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('布局文件格式无效');
 const data = value as Record<string, unknown>;
 if (data.version !== 1 || !data.positions || typeof data.positions !== 'object' || Array.isArray(data.positions) || !data.roles || typeof data.roles !== 'object' || Array.isArray(data.roles)) throw new Error('不支持的布局格式');
 const positions: Layout = Object.create(null), roles: Record<string, Role> = Object.create(null);
 for (const [id, raw] of Object.entries(data.positions)) {
  if (!id || !raw || typeof raw !== 'object' || Array.isArray(raw)) throw new Error(`无效的节点位置：${id}`);
  const p = raw as Record<string, unknown>;
  if (typeof p.x !== 'number' || !Number.isFinite(p.x) || typeof p.y !== 'number' || !Number.isFinite(p.y)) throw new Error(`无效的节点位置：${id}`);
  positions[id] = {x:p.x,y:p.y};
 }
 for (const [id, role] of Object.entries(data.roles)) {
  if (!id || typeof role !== 'string' || !Object.hasOwn(designs, role)) throw new Error(`无效的节点类型：${id}`);
  roles[id] = role as Role;
 }
 return {version:1, positions, roles};
}
