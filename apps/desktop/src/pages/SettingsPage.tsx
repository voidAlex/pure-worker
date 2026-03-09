/**
 * 系统设置页面
 *
 * 包含五个标签页：
 * 1. AI 配置 - AI 服务商/模型管理、参数预设
 * 2. 安全隐私 - 存储生命周期、脱敏开关
 * 3. 模板导出 - 模板文件管理、默认导出格式
 * 4. 快捷键 - 全局快捷键管理
 * 5. Skills 与 MCP - 技能管理、uv 环境、MCP 服务器管理
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  commands,
  type AppError,
  type AiConfigSafe,
  type CreateAiConfigInput,
  type UpdateAiConfigInput,
  type AiParamPreset,
  type CreatePresetInput,
  type UpdatePresetInput,
  type GlobalShortcut,
  type CreateGlobalShortcutInput,
  type UpdateGlobalShortcutInput,
  type SkillRecord,
  type CreateSkillInput,
  type UpdateSkillInput,
  type McpServerRecord,
  type CreateMcpServerInput,
  type UpdateMcpServerInput,
  type TemplateFile,
  type CreateTemplateFileInput,
  type StorageStats,
  type UvHealthResult,
} from '@/bindings';
import { useToast } from '@/hooks/useToast';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import { EmptyState } from '@/components/shared/EmptyState';
import {
  Cpu,
  Shield,
  FileText,
  Keyboard,
  Puzzle,
  Plus,
  Edit2,
  Trash2,
  Settings,
  Eye,
  EyeOff,
  CheckCircle2,
  XCircle,
  Activity,
  Download,
  Archive,
  AlertTriangle,
  Play,
  RefreshCw,
} from 'lucide-react';

/* ───────────────────────── 工具函数 ───────────────────────── */

/** 从 AppError 联合类型中提取错误信息字符串 */
const getErrMsg = (err: AppError): string => {
  const values = Object.values(err as Record<string, string>);
  return values[0] ?? '未知错误';
};

/* ───────────────────────── 标签页定义 ───────────────────────── */

/** 标签页配置项 */
interface TabDef {
  key: string;
  label: string;
  icon: React.ReactNode;
}

/** 五个标签页配置 */
const TABS: TabDef[] = [
  { key: 'ai', label: 'AI 配置', icon: <Cpu size={16} /> },
  { key: 'security', label: '安全隐私', icon: <Shield size={16} /> },
  { key: 'template', label: '模板导出', icon: <FileText size={16} /> },
  { key: 'shortcut', label: '快捷键', icon: <Keyboard size={16} /> },
  { key: 'skills', label: 'Skills 与 MCP', icon: <Puzzle size={16} /> },
];

/* ═══════════════════════════════════════════════════════════════
   1. AI 配置标签页
   ═══════════════════════════════════════════════════════════════ */

/** AI 配置标签页 — 服务商/模型管理 + 参数预设 */
const AiConfigTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error: showError } = useToast();

  /* ── AI 配置表单 ── */
  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [form, setForm] = useState({
    provider_name: '',
    display_name: '',
    base_url: '',
    api_key: '',
    default_model: '',
    is_active: true,
    config_json: '',
  });
  const [showApiKey, setShowApiKey] = useState(false);

  /* ── 预设表单 ── */
  const [showPresetForm, setShowPresetForm] = useState(false);
  const [editPresetId, setEditPresetId] = useState<string | null>(null);
  const [presetForm, setPresetForm] = useState({
    name: '',
    display_name: '',
    temperature: 0.7,
    top_p: 0.9 as number | null,
    max_tokens: 2048 as number | null,
  });

  /* ── 删除确认 ── */
  const [deleteTarget, setDeleteTarget] = useState<{ type: 'config' | 'preset'; id: string; name: string } | null>(null);

  /* ── 数据查询 ── */
  const configsQuery = useQuery({
    queryKey: ['ai-configs'],
    queryFn: async () => {
      const r = await commands.listAiConfigs();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  const presetsQuery = useQuery({
    queryKey: ['ai-presets'],
    queryFn: async () => {
      const r = await commands.listAiParamPresets();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  /* ── Mutations: AI 配置 ── */
  const createConfig = useMutation({
    mutationFn: async (input: CreateAiConfigInput) => {
      const r = await commands.createAiConfig(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-configs'] }); success('AI 配置已创建'); resetForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const updateConfig = useMutation({
    mutationFn: async (input: UpdateAiConfigInput) => {
      const r = await commands.updateAiConfig(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-configs'] }); success('AI 配置已更新'); resetForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const deleteConfig = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.deleteAiConfig({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-configs'] }); success('AI 配置已删除'); },
    onError: (e: Error) => showError(e.message),
  });

  /* ── Mutations: 参数预设 ── */
  const createPreset = useMutation({
    mutationFn: async (input: CreatePresetInput) => {
      const r = await commands.createAiParamPreset(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-presets'] }); success('参数预设已创建'); resetPresetForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const updatePreset = useMutation({
    mutationFn: async (input: UpdatePresetInput) => {
      const r = await commands.updateAiParamPreset(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-presets'] }); success('参数预设已更新'); resetPresetForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const deletePreset = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.deleteAiParamPreset({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-presets'] }); success('参数预设已删除'); },
    onError: (e: Error) => showError(e.message),
  });

  const activatePreset = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.activateAiParamPreset({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['ai-presets'] }); success('预设已激活'); },
    onError: (e: Error) => showError(e.message),
  });

  /** 重置 AI 配置表单 */
  const resetForm = () => {
    setShowForm(false);
    setEditId(null);
    setForm({ provider_name: '', display_name: '', base_url: '', api_key: '', default_model: '', is_active: true, config_json: '' });
    setShowApiKey(false);
  };

  /** 重置预设表单 */
  const resetPresetForm = () => {
    setShowPresetForm(false);
    setEditPresetId(null);
    setPresetForm({ name: '', display_name: '', temperature: 0.7, top_p: 0.9, max_tokens: 2048 });
  };

  /** 进入编辑 AI 配置模式 */
  const startEditConfig = (cfg: AiConfigSafe) => {
    setEditId(cfg.id);
    setForm({
      provider_name: cfg.provider_name,
      display_name: cfg.display_name,
      base_url: cfg.base_url,
      api_key: '',
      default_model: cfg.default_model,
      is_active: cfg.is_active === 1,
      config_json: cfg.config_json ?? '',
    });
    setShowForm(true);
  };

  /** 进入编辑预设模式 */
  const startEditPreset = (p: AiParamPreset) => {
    setEditPresetId(p.id);
    setPresetForm({
      name: p.name,
      display_name: p.display_name,
      temperature: p.temperature,
      top_p: p.top_p,
      max_tokens: p.max_tokens,
    });
    setShowPresetForm(true);
  };

  /** 提交 AI 配置 */
  const handleSubmitConfig = () => {
    if (editId) {
      updateConfig.mutate({
        id: editId,
        display_name: form.display_name || null,
        base_url: form.base_url || null,
        api_key: form.api_key || null,
        default_model: form.default_model || null,
        is_active: form.is_active,
        config_json: form.config_json || null,
      });
    } else {
      createConfig.mutate({
        provider_name: form.provider_name,
        display_name: form.display_name,
        base_url: form.base_url,
        api_key: form.api_key,
        default_model: form.default_model,
        is_active: form.is_active,
        config_json: form.config_json || null,
      });
    }
  };

  /** 提交预设 */
  const handleSubmitPreset = () => {
    if (editPresetId) {
      updatePreset.mutate({
        id: editPresetId,
        name: presetForm.name || null,
        display_name: presetForm.display_name || null,
        temperature: presetForm.temperature,
        top_p: presetForm.top_p,
        max_tokens: presetForm.max_tokens,
        is_default: null,
        is_active: null,
      });
    } else {
      createPreset.mutate({
        name: presetForm.name,
        display_name: presetForm.display_name,
        temperature: presetForm.temperature,
        top_p: presetForm.top_p,
        max_tokens: presetForm.max_tokens,
        is_default: null,
        is_active: null,
      });
    }
  };

  return (
    <div className="space-y-6">
      {/* ── AI 服务商/模型列表 ── */}
      <section>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-lg font-semibold">AI 服务商 / 模型</h3>
          <button className="flex items-center gap-1 px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => { resetForm(); setShowForm(true); }}>
            <Plus size={14} /> 添加配置
          </button>
        </div>

        {/* 内联表单 */}
        {showForm && (
          <div className="p-4 mb-4 border rounded-lg bg-gray-50 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <input className="border rounded px-3 py-2 text-sm" placeholder="服务商标识（如 deepseek）" value={form.provider_name} onChange={(e) => setForm({ ...form, provider_name: e.target.value })} disabled={!!editId} />
              <input className="border rounded px-3 py-2 text-sm" placeholder="显示名称（如 DeepSeek V3）" value={form.display_name} onChange={(e) => setForm({ ...form, display_name: e.target.value })} />
              <input className="border rounded px-3 py-2 text-sm" placeholder="API 地址" value={form.base_url} onChange={(e) => setForm({ ...form, base_url: e.target.value })} />
              <div className="relative">
                <input className="w-full border rounded px-3 py-2 text-sm pr-8" type={showApiKey ? 'text' : 'password'} placeholder={editId ? 'API 密钥（留空不修改）' : 'API 密钥'} value={form.api_key} onChange={(e) => setForm({ ...form, api_key: e.target.value })} />
                <button type="button" className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600" onClick={() => setShowApiKey(!showApiKey)}>
                  {showApiKey ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
              </div>
            </div>
            <input className="w-full border rounded px-3 py-2 text-sm" placeholder="默认模型（如 deepseek-chat）" value={form.default_model} onChange={(e) => setForm({ ...form, default_model: e.target.value })} />
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={form.is_active} onChange={(e) => setForm({ ...form, is_active: e.target.checked })} />
              启用
            </label>
            <div className="flex gap-2">
              <button className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={handleSubmitConfig}>{editId ? '更新' : '创建'}</button>
              <button className="px-3 py-1.5 text-sm border rounded hover:bg-gray-100" onClick={resetForm}>取消</button>
            </div>
          </div>
        )}

        {/* 配置列表 */}
        {configsQuery.data?.length ? (
          <div className="space-y-2">
            {configsQuery.data.map((cfg) => (
              <div key={cfg.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
                <div>
                  <span className="font-medium">{cfg.display_name}</span>
                  <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">{cfg.provider_name}</span>
                  <span className="ml-2 text-xs text-gray-400">{cfg.default_model}</span>
                  {cfg.is_active === 1 && <span className="ml-2 px-2 py-0.5 text-xs bg-green-100 text-green-700 rounded">激活</span>}
                  {cfg.has_api_key && <span className="ml-2 text-xs text-green-600">密钥已配置</span>}
                </div>
                <div className="flex gap-2">
                  <button className="p-1.5 hover:bg-gray-200 rounded" title="编辑配置" onClick={() => startEditConfig(cfg)}><Edit2 size={14} /></button>
                  <button className="p-1.5 hover:bg-red-100 rounded text-red-500" title="删除配置" onClick={() => setDeleteTarget({ type: 'config', id: cfg.id, name: cfg.display_name })}><Trash2 size={14} /></button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState title="暂无 AI 配置" description={'点击「添加配置」添加您的第一个 AI 服务商'} icon={<Cpu size={32} className="text-gray-400" />} />
        )}
      </section>

      {/* ── 参数预设 ── */}
      <section>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-lg font-semibold">参数预设</h3>
          <button className="flex items-center gap-1 px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => { resetPresetForm(); setShowPresetForm(true); }}>
            <Plus size={14} /> 添加预设
          </button>
        </div>

        {showPresetForm && (
          <div className="p-4 mb-4 border rounded-lg bg-gray-50 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <input className="border rounded px-3 py-2 text-sm" placeholder="预设标识（如 strict）" value={presetForm.name} onChange={(e) => setPresetForm({ ...presetForm, name: e.target.value })} />
              <input className="border rounded px-3 py-2 text-sm" placeholder="显示名称（如 严谨模式）" value={presetForm.display_name} onChange={(e) => setPresetForm({ ...presetForm, display_name: e.target.value })} />
            </div>
            <div className="grid grid-cols-3 gap-3">
              <label className="text-sm">
                Temperature
                <input className="w-full border rounded px-3 py-2 text-sm mt-1" type="number" step={0.1} min={0} max={2} value={presetForm.temperature} onChange={(e) => setPresetForm({ ...presetForm, temperature: parseFloat(e.target.value) || 0 })} />
              </label>
              <label className="text-sm">
                Top P
                <input className="w-full border rounded px-3 py-2 text-sm mt-1" type="number" step={0.1} min={0} max={1} value={presetForm.top_p ?? ''} onChange={(e) => setPresetForm({ ...presetForm, top_p: e.target.value ? parseFloat(e.target.value) : null })} />
              </label>
              <label className="text-sm">
                Max Tokens
                <input className="w-full border rounded px-3 py-2 text-sm mt-1" type="number" step={256} min={256} max={128000} value={presetForm.max_tokens ?? ''} onChange={(e) => setPresetForm({ ...presetForm, max_tokens: e.target.value ? parseInt(e.target.value) : null })} />
              </label>
            </div>
            <div className="flex gap-2">
              <button className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={handleSubmitPreset}>{editPresetId ? '更新' : '创建'}</button>
              <button className="px-3 py-1.5 text-sm border rounded hover:bg-gray-100" onClick={resetPresetForm}>取消</button>
            </div>
          </div>
        )}

        {presetsQuery.data?.length ? (
          <div className="space-y-2">
            {presetsQuery.data.map((p) => (
              <div key={p.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
                <div>
                  <span className="font-medium">{p.display_name}</span>
                  <span className="ml-2 text-xs text-gray-400">({p.name})</span>
                  <span className="ml-3 text-xs text-gray-500">T={p.temperature} P={p.top_p ?? '-'} MaxTok={p.max_tokens ?? '-'}</span>
                  {p.is_active === 1 && <span className="ml-2 px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded">激活中</span>}
                  {p.is_default === 1 && <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">默认</span>}
                </div>
                <div className="flex gap-2">
                  {p.is_active !== 1 && (
                    <button className="p-1.5 hover:bg-blue-100 rounded text-blue-600" title="激活预设" onClick={() => activatePreset.mutate(p.id)}><Play size={14} /></button>
                  )}
                  <button className="p-1.5 hover:bg-gray-200 rounded" title="编辑预设" onClick={() => startEditPreset(p)}><Edit2 size={14} /></button>
                  <button className="p-1.5 hover:bg-red-100 rounded text-red-500" title="删除预设" onClick={() => setDeleteTarget({ type: 'preset', id: p.id, name: p.display_name })}><Trash2 size={14} /></button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState title="暂无参数预设" description="添加严谨/创意/均衡等预设以快速切换 AI 参数" icon={<Settings size={32} className="text-gray-400" />} />
        )}
      </section>

      {/* 删除确认弹窗 */}
      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除"
        message={`确定要删除「${deleteTarget?.name ?? ''}」吗？此操作不可撤销。`}
        confirmText="删除"
        isDestructive
        onConfirm={() => {
          if (!deleteTarget) return;
          if (deleteTarget.type === 'config') deleteConfig.mutate(deleteTarget.id);
          else deletePreset.mutate(deleteTarget.id);
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════
   2. 安全隐私标签页
   ═══════════════════════════════════════════════════════════════ */

/** 安全隐私标签页 — 存储生命周期 + 脱敏开关 */
const SecurityTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error: showError } = useToast();
  const [confirmAction, setConfirmAction] = useState<'export' | 'archive' | 'erase' | null>(null);

  /** 存储统计查询 */
  const statsQuery = useQuery({
    queryKey: ['storage-stats'],
    queryFn: async () => {
      const r = await commands.getStorageStats();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  /** 脱敏开关查询 */
  const desensitizeQuery = useQuery({
    queryKey: ['setting', 'desensitize_enabled'],
    queryFn: async () => {
      const r = await commands.getSetting('desensitize_enabled');
      if (r.status === 'ok') return r.data.value === 'true';
      return true;
    },
  });

  /** 切换脱敏开关 */
  const toggleDesensitize = useMutation({
    mutationFn: async (enabled: boolean) => {
      const r = await commands.updateSetting('desensitize_enabled', String(enabled), 'security', '外发前脱敏开关');
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['setting', 'desensitize_enabled'] }); success('脱敏设置已更新'); },
    onError: (e: Error) => showError(e.message),
  });

  /** 导出工作区 */
  const exportWs = useMutation({
    mutationFn: async () => {
      const r = await commands.exportWorkspace({ output_path: 'workspace_export.zip', approved: true });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: (data) => { success(`工作区已导出至 ${data.output_path}`); queryClient.invalidateQueries({ queryKey: ['storage-stats'] }); },
    onError: (e: Error) => showError(e.message),
  });

  /** 归档工作区 */
  const archiveWs = useMutation({
    mutationFn: async () => {
      const r = await commands.archiveWorkspace({ archive_name: null });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: (data) => { success(`工作区已归档至 ${data.archive_path}`); queryClient.invalidateQueries({ queryKey: ['storage-stats'] }); },
    onError: (e: Error) => showError(e.message),
  });

  /** 清除工作区数据 */
  const eraseWs = useMutation({
    mutationFn: async () => {
      const r = await commands.eraseWorkspace({ approved: true });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { success('工作区数据已清除'); queryClient.invalidateQueries({ queryKey: ['storage-stats'] }); },
    onError: (e: Error) => showError(e.message),
  });

  const stats = statsQuery.data;

  return (
    <div className="space-y-6">
      {/* ── 存储统计 ── */}
      <section>
        <h3 className="text-lg font-semibold mb-3">存储统计</h3>
        {stats ? (
          <div className="grid grid-cols-2 gap-4">
            <div className="p-4 border rounded-lg">
              <div className="text-sm text-gray-500">工作区路径</div>
              <div className="font-mono text-sm mt-1 truncate" title={stats.workspace_path}>{stats.workspace_path}</div>
            </div>
            <div className="p-4 border rounded-lg">
              <div className="text-sm text-gray-500">文件数 / 总大小</div>
              <div className="text-lg font-semibold mt-1">{stats.total_files} 个文件 / {stats.total_size_display}</div>
            </div>
            <div className="p-4 border rounded-lg">
              <div className="text-sm text-gray-500">归档数量</div>
              <div className="text-lg font-semibold mt-1">{stats.archive_count}</div>
            </div>
          </div>
        ) : (
          <div className="text-sm text-gray-400">加载中...</div>
        )}
      </section>

      {/* ── 脱敏开关 ── */}
      <section>
        <h3 className="text-lg font-semibold mb-3">发送前脱敏</h3>
        <div className="flex items-center gap-3 p-4 border rounded-lg">
          <button
            className={`relative w-12 h-6 rounded-full transition-colors ${desensitizeQuery.data ? 'bg-blue-600' : 'bg-gray-300'}`}
            onClick={() => toggleDesensitize.mutate(!desensitizeQuery.data)}
            title={desensitizeQuery.data ? '点击关闭脱敏' : '点击开启脱敏'}
          >
            <span className={`absolute top-0.5 w-5 h-5 bg-white rounded-full transition-transform ${desensitizeQuery.data ? 'translate-x-6' : 'translate-x-0.5'}`} />
          </button>
          <div>
            <div className="flex items-center gap-2">
              {desensitizeQuery.data ? <Eye size={16} className="text-blue-600" /> : <EyeOff size={16} className="text-gray-400" />}
              <span className="text-sm font-medium">{desensitizeQuery.data ? '脱敏已开启' : '脱敏已关闭'}</span>
            </div>
            <p className="text-xs text-gray-500 mt-1">开启后，发送给 AI 的内容会自动替换手机号、身份证号、邮箱等敏感信息</p>
          </div>
        </div>
      </section>

      {/* ── 数据生命周期 ── */}
      <section>
        <h3 className="text-lg font-semibold mb-3">数据生命周期</h3>
        <div className="flex gap-3">
          <button className="flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-gray-50 text-sm" onClick={() => setConfirmAction('export')}>
            <Download size={16} /> 导出工作区
          </button>
          <button className="flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-gray-50 text-sm" onClick={() => setConfirmAction('archive')}>
            <Archive size={16} /> 归档工作区
          </button>
          <button className="flex items-center gap-2 px-4 py-2 border border-red-200 rounded-lg hover:bg-red-50 text-sm text-red-600" onClick={() => setConfirmAction('erase')}>
            <AlertTriangle size={16} /> 清除数据
          </button>
        </div>
      </section>

      <ConfirmDialog
        isOpen={confirmAction !== null}
        title={confirmAction === 'erase' ? '确认清除所有数据' : confirmAction === 'archive' ? '确认归档工作区' : '确认导出工作区'}
        message={
          confirmAction === 'erase'
            ? '此操作将永久删除工作区中的所有数据，且不可恢复。请确认您已做好备份。'
            : confirmAction === 'archive'
              ? '归档会将当前工作区数据打包存档，您可以稍后恢复。'
              : '导出会将工作区数据打包为 ZIP 文件。'
        }
        confirmText={confirmAction === 'erase' ? '确认清除' : '确认'}
        isDestructive={confirmAction === 'erase'}
        onConfirm={() => {
          if (confirmAction === 'export') exportWs.mutate();
          else if (confirmAction === 'archive') archiveWs.mutate();
          else if (confirmAction === 'erase') eraseWs.mutate();
          setConfirmAction(null);
        }}
        onCancel={() => setConfirmAction(null)}
      />
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════
   3. 模板导出标签页
   ═══════════════════════════════════════════════════════════════ */

/** 模板导出标签页 — 模板文件管理 + 默认导出格式 */
const TemplateTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error: showError } = useToast();

  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState({
    type: 'docx',
    school_scope: '' as string | null,
    version: '' as string | null,
    file_path: '',
  });
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; label: string } | null>(null);

  /** 模板列表查询 */
  const templatesQuery = useQuery({
    queryKey: ['template-files'],
    queryFn: async () => {
      const r = await commands.listTemplateFiles({ type: null, enabled: null });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  /** 默认导出格式查询 */
  const exportFormatQuery = useQuery({
    queryKey: ['setting', 'default_export_format'],
    queryFn: async () => {
      const r = await commands.getSetting('default_export_format');
      if (r.status === 'ok') return r.data.value;
      return 'docx';
    },
  });

  /** 更新默认导出格式 */
  const updateExportFormat = useMutation({
    mutationFn: async (fmt: string) => {
      const r = await commands.updateSetting('default_export_format', fmt, 'export', '默认导出格式');
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['setting', 'default_export_format'] }); success('默认导出格式已更新'); },
    onError: (e: Error) => showError(e.message),
  });

  /** 创建模板 */
  const createTemplate = useMutation({
    mutationFn: async (input: CreateTemplateFileInput) => {
      const r = await commands.createTemplateFile(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['template-files'] });
      success('模板已创建');
      setShowForm(false);
      setForm({ type: 'docx', school_scope: '', version: '', file_path: '' });
    },
    onError: (e: Error) => showError(e.message),
  });

  /** 删除模板 */
  const deleteTemplate = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.deleteTemplateFile({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['template-files'] }); success('模板已删除'); },
    onError: (e: Error) => showError(e.message),
  });

  return (
    <div className="space-y-6">
      {/* ── 默认导出格式 ── */}
      <section>
        <h3 className="text-lg font-semibold mb-3">默认导出格式</h3>
        <div className="flex gap-3">
          {['docx', 'pdf', 'txt', 'markdown'].map((fmt) => (
            <button
              key={fmt}
              className={`px-4 py-2 text-sm border rounded-lg ${exportFormatQuery.data === fmt ? 'bg-blue-600 text-white border-blue-600' : 'hover:bg-gray-50'}`}
              onClick={() => updateExportFormat.mutate(fmt)}
            >
              {fmt.toUpperCase()}
            </button>
          ))}
        </div>
      </section>

      {/* ── 模板文件管理 ── */}
      <section>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-lg font-semibold">模板文件</h3>
          <button className="flex items-center gap-1 px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => setShowForm(true)}>
            <Plus size={14} /> 添加模板
          </button>
        </div>

        {showForm && (
          <div className="p-4 mb-4 border rounded-lg bg-gray-50 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <select className="border rounded px-3 py-2 text-sm" value={form.type} onChange={(e) => setForm({ ...form, type: e.target.value })}>
                <option value="docx">DOCX</option>
                <option value="pdf">PDF</option>
                <option value="txt">TXT</option>
                <option value="markdown">Markdown</option>
              </select>
              <input className="border rounded px-3 py-2 text-sm" placeholder="适用范围（可选）" value={form.school_scope ?? ''} onChange={(e) => setForm({ ...form, school_scope: e.target.value || null })} />
            </div>
            <input className="w-full border rounded px-3 py-2 text-sm" placeholder="文件路径" value={form.file_path} onChange={(e) => setForm({ ...form, file_path: e.target.value })} />
            <input className="w-full border rounded px-3 py-2 text-sm" placeholder="版本号（可选）" value={form.version ?? ''} onChange={(e) => setForm({ ...form, version: e.target.value || null })} />
            <div className="flex gap-2">
              <button className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => createTemplate.mutate({ type: form.type, school_scope: form.school_scope || null, version: form.version || null, file_path: form.file_path, enabled: 1 })}>创建</button>
              <button className="px-3 py-1.5 text-sm border rounded hover:bg-gray-100" onClick={() => setShowForm(false)}>取消</button>
            </div>
          </div>
        )}

        {templatesQuery.data?.length ? (
          <div className="space-y-2">
            {templatesQuery.data.map((tpl) => (
              <div key={tpl.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
                <div>
                  <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">{tpl.type.toUpperCase()}</span>
                  <span className="ml-2 font-mono text-sm text-gray-700">{tpl.file_path}</span>
                  {tpl.version && <span className="ml-2 text-xs text-gray-400">v{tpl.version}</span>}
                  {tpl.school_scope && <span className="ml-2 text-xs text-gray-400">{tpl.school_scope}</span>}
                </div>
                <button className="p-1.5 hover:bg-red-100 rounded text-red-500" title="删除模板" onClick={() => setDeleteTarget({ id: tpl.id, label: `${tpl.type} - ${tpl.file_path}` })}><Trash2 size={14} /></button>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState title="暂无模板" description="添加文档模板以快速导出" icon={<FileText size={32} className="text-gray-400" />} />
        )}
      </section>

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除模板"
        message={`确定要删除模板「${deleteTarget?.label ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={() => { if (deleteTarget) deleteTemplate.mutate(deleteTarget.id); setDeleteTarget(null); }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════
   4. 快捷键标签页
   ═══════════════════════════════════════════════════════════════ */

/** 快捷键标签页 — 全局快捷键管理 */
const ShortcutTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error: showError } = useToast();

  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [form, setForm] = useState({
    action: '',
    key_combination: '',
    description: '' as string | null,
    enabled: 1 as number | null,
  });
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; label: string } | null>(null);

  /** 快捷键列表查询 */
  const shortcutsQuery = useQuery({
    queryKey: ['global-shortcuts'],
    queryFn: async () => {
      const r = await commands.listGlobalShortcuts();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  /** 创建快捷键 */
  const createShortcut = useMutation({
    mutationFn: async (input: CreateGlobalShortcutInput) => {
      const r = await commands.createGlobalShortcut(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['global-shortcuts'] }); success('快捷键已创建'); resetForm(); },
    onError: (e: Error) => showError(e.message),
  });

  /** 更新快捷键 */
  const updateShortcut = useMutation({
    mutationFn: async (input: UpdateGlobalShortcutInput) => {
      const r = await commands.updateGlobalShortcut(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['global-shortcuts'] }); success('快捷键已更新'); resetForm(); },
    onError: (e: Error) => showError(e.message),
  });

  /** 删除快捷键 */
  const deleteShortcut = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.deleteGlobalShortcut({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['global-shortcuts'] }); success('快捷键已删除'); },
    onError: (e: Error) => showError(e.message),
  });

  /** 重置表单 */
  const resetForm = () => {
    setShowForm(false);
    setEditId(null);
    setForm({ action: '', key_combination: '', description: '', enabled: 1 });
  };

  /** 进入编辑模式 */
  const startEdit = (s: GlobalShortcut) => {
    setEditId(s.id);
    setForm({ action: s.action, key_combination: s.key_combination, description: s.description, enabled: s.enabled });
    setShowForm(true);
  };

  /** 提交表单 */
  const handleSubmit = () => {
    if (editId) {
      updateShortcut.mutate({ id: editId, action: form.action || null, key_combination: form.key_combination || null, enabled: form.enabled, description: form.description || null });
    } else {
      createShortcut.mutate({ action: form.action, key_combination: form.key_combination, enabled: form.enabled, description: form.description || null });
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-lg font-semibold">全局快捷键</h3>
        <button className="flex items-center gap-1 px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => { resetForm(); setShowForm(true); }}>
          <Plus size={14} /> 添加快捷键
        </button>
      </div>

      {showForm && (
        <div className="p-4 mb-4 border rounded-lg bg-gray-50 space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <input className="border rounded px-3 py-2 text-sm" placeholder="动作标识（如 create_task）" value={form.action} onChange={(e) => setForm({ ...form, action: e.target.value })} />
            <input className="border rounded px-3 py-2 text-sm" placeholder="按键组合（如 Ctrl+Shift+N）" value={form.key_combination} onChange={(e) => setForm({ ...form, key_combination: e.target.value })} />
          </div>
          <input className="w-full border rounded px-3 py-2 text-sm" placeholder="描述（可选）" value={form.description ?? ''} onChange={(e) => setForm({ ...form, description: e.target.value || null })} />
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={form.enabled === 1} onChange={(e) => setForm({ ...form, enabled: e.target.checked ? 1 : 0 })} />
            启用
          </label>
          <div className="flex gap-2">
            <button className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={handleSubmit}>{editId ? '更新' : '创建'}</button>
            <button className="px-3 py-1.5 text-sm border rounded hover:bg-gray-100" onClick={resetForm}>取消</button>
          </div>
        </div>
      )}

      {shortcutsQuery.data?.length ? (
        <div className="space-y-2">
          {shortcutsQuery.data.map((s) => (
            <div key={s.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
              <div className="flex items-center gap-3">
                {s.enabled === 1 ? <CheckCircle2 size={16} className="text-green-500" /> : <XCircle size={16} className="text-gray-400" />}
                <span className="font-medium">{s.action}</span>
                <kbd className="px-2 py-0.5 text-xs bg-gray-100 border rounded font-mono">{s.key_combination}</kbd>
                {s.description && <span className="text-xs text-gray-400">{s.description}</span>}
              </div>
              <div className="flex gap-2">
                <button className="p-1.5 hover:bg-gray-200 rounded" title="编辑快捷键" onClick={() => startEdit(s)}><Edit2 size={14} /></button>
                <button className="p-1.5 hover:bg-red-100 rounded text-red-500" title="删除快捷键" onClick={() => setDeleteTarget({ id: s.id, label: s.action })}><Trash2 size={14} /></button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <EmptyState title="暂无快捷键" description="添加全局快捷键以提升操作效率" icon={<Keyboard size={32} className="text-gray-400" />} />
      )}

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除快捷键"
        message={`确定要删除快捷键「${deleteTarget?.label ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={() => { if (deleteTarget) deleteShortcut.mutate(deleteTarget.id); setDeleteTarget(null); }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════
   5. Skills 与 MCP 标签页
   ═══════════════════════════════════════════════════════════════ */

/** Skills 与 MCP 标签页 — 技能管理 / uv 环境 / MCP 服务器 */
const SkillsMcpTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error: showError } = useToast();
  const [subTab, setSubTab] = useState<'skills' | 'uv' | 'mcp'>('skills');

  /* ── Skills 表单 ── */
  const [showSkillForm, setShowSkillForm] = useState(false);
  const [editSkillId, setEditSkillId] = useState<string | null>(null);
  const [skillForm, setSkillForm] = useState({
    name: '',
    display_name: '' as string | null,
    description: '' as string | null,
    skill_type: 'builtin',
    version: '' as string | null,
    source: '' as string | null,
    permission_scope: '' as string | null,
    config_json: '' as string | null,
  });

  /* ── MCP 表单 ── */
  const [showMcpForm, setShowMcpForm] = useState(false);
  const [editMcpId, setEditMcpId] = useState<string | null>(null);
  const [mcpForm, setMcpForm] = useState({
    name: '',
    display_name: '' as string | null,
    description: '' as string | null,
    transport: 'stdio',
    command: '' as string | null,
    args_json: '' as string | null,
    env_json: '' as string | null,
    permission_scope: '' as string | null,
  });

  /* ── 删除确认 ── */
  const [deleteTarget, setDeleteTarget] = useState<{ type: 'skill' | 'mcp'; id: string; name: string } | null>(null);

  /* ── 查询 ── */
  const skillsQuery = useQuery({
    queryKey: ['skills'],
    queryFn: async () => {
      const r = await commands.listSkills();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  const mcpQuery = useQuery({
    queryKey: ['mcp-servers'],
    queryFn: async () => {
      const r = await commands.listMcpServers();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  const uvHealthQuery = useQuery({
    queryKey: ['uv-health'],
    queryFn: async () => {
      const r = await commands.checkUvHealth();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
  });

  /* ── Skills Mutations ── */
  const createSkill = useMutation({
    mutationFn: async (input: CreateSkillInput) => {
      const r = await commands.createSkill(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['skills'] }); success('技能已创建'); resetSkillForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const updateSkillMut = useMutation({
    mutationFn: async ({ id, input }: { id: string; input: UpdateSkillInput }) => {
      const r = await commands.updateSkill(id, input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['skills'] }); success('技能已更新'); resetSkillForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const deleteSkillMut = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.deleteSkill({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['skills'] }); success('技能已删除'); },
    onError: (e: Error) => showError(e.message),
  });

  const checkSkill = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.checkSkillHealth(id);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: (data) => {
      if (data.health_status === 'healthy') success(`技能「${data.name}」健康检查通过`);
      else showError(`技能「${data.name}」异常：${data.message}`);
      queryClient.invalidateQueries({ queryKey: ['skills'] });
    },
    onError: (e: Error) => showError(e.message),
  });

  /* ── uv Mutations ── */
  const installUv = useMutation({
    mutationFn: async () => {
      const r = await commands.installUv();
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: (data) => {
      if (data.success) success(`uv 安装成功，版本: ${data.version ?? '未知'}`);
      else showError(`uv 安装失败：${data.output}`);
      queryClient.invalidateQueries({ queryKey: ['uv-health'] });
    },
    onError: (e: Error) => showError(e.message),
  });

  /* ── MCP Mutations ── */
  const createMcp = useMutation({
    mutationFn: async (input: CreateMcpServerInput) => {
      const r = await commands.createMcpServer(input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['mcp-servers'] }); success('MCP 服务器已创建'); resetMcpForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const updateMcpMut = useMutation({
    mutationFn: async ({ id, input }: { id: string; input: UpdateMcpServerInput }) => {
      const r = await commands.updateMcpServer(id, input);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['mcp-servers'] }); success('MCP 服务器已更新'); resetMcpForm(); },
    onError: (e: Error) => showError(e.message),
  });

  const deleteMcpMut = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.deleteMcpServer({ id });
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['mcp-servers'] }); success('MCP 服务器已删除'); },
    onError: (e: Error) => showError(e.message),
  });

  const checkMcp = useMutation({
    mutationFn: async (id: string) => {
      const r = await commands.checkMcpHealth(id);
      if (r.status === 'error') throw new Error(JSON.stringify(r.error));
      return r.data;
    },
    onSuccess: (data) => {
      if (data.health_status === 'healthy') success(`MCP「${data.name}」健康检查通过`);
      else showError(`MCP「${data.name}」异常：${data.message}`);
      queryClient.invalidateQueries({ queryKey: ['mcp-servers'] });
    },
    onError: (e: Error) => showError(e.message),
  });

  /** 重置 Skill 表单 */
  const resetSkillForm = () => {
    setShowSkillForm(false);
    setEditSkillId(null);
    setSkillForm({ name: '', display_name: '', description: '', skill_type: 'builtin', version: '', source: '', permission_scope: '', config_json: '' });
  };

  /** 重置 MCP 表单 */
  const resetMcpForm = () => {
    setShowMcpForm(false);
    setEditMcpId(null);
    setMcpForm({ name: '', display_name: '', description: '', transport: 'stdio', command: '', args_json: '', env_json: '', permission_scope: '' });
  };

  /** 进入编辑 Skill 模式 */
  const startEditSkill = (s: SkillRecord) => {
    setEditSkillId(s.id);
    setSkillForm({
      name: s.name,
      display_name: s.display_name,
      description: s.description,
      skill_type: s.skill_type,
      version: s.version,
      source: s.source,
      permission_scope: s.permission_scope,
      config_json: s.config_json,
    });
    setShowSkillForm(true);
  };

  /** 进入编辑 MCP 模式 */
  const startEditMcp = (m: McpServerRecord) => {
    setEditMcpId(m.id);
    setMcpForm({
      name: m.name,
      display_name: m.display_name,
      description: m.description,
      transport: m.transport,
      command: m.command,
      args_json: m.args_json,
      env_json: m.env_json,
      permission_scope: m.permission_scope,
    });
    setShowMcpForm(true);
  };

  /** 提交 Skill */
  const handleSubmitSkill = () => {
    if (editSkillId) {
      updateSkillMut.mutate({
        id: editSkillId,
        input: {
          display_name: skillForm.display_name || null,
          description: skillForm.description || null,
          permission_scope: skillForm.permission_scope || null,
          config_json: skillForm.config_json || null,
          status: null,
        },
      });
    } else {
      createSkill.mutate({
        name: skillForm.name,
        version: skillForm.version || null,
        source: skillForm.source || null,
        permission_scope: skillForm.permission_scope || null,
        display_name: skillForm.display_name || null,
        description: skillForm.description || null,
        skill_type: skillForm.skill_type,
        config_json: skillForm.config_json || null,
      });
    }
  };

  /** 提交 MCP */
  const handleSubmitMcp = () => {
    if (editMcpId) {
      updateMcpMut.mutate({
        id: editMcpId,
        input: {
          display_name: mcpForm.display_name || null,
          description: mcpForm.description || null,
          command: mcpForm.command || null,
          args_json: mcpForm.args_json || null,
          env_json: mcpForm.env_json || null,
          permission_scope: mcpForm.permission_scope || null,
          enabled: null,
        },
      });
    } else {
      createMcp.mutate({
        name: mcpForm.name,
        transport: mcpForm.transport,
        command: mcpForm.command || null,
        args_json: mcpForm.args_json || null,
        env_json: mcpForm.env_json || null,
        permission_scope: mcpForm.permission_scope || null,
        display_name: mcpForm.display_name || null,
        description: mcpForm.description || null,
      });
    }
  };

  const uvHealth = uvHealthQuery.data;

  return (
    <div className="space-y-4">
      {/* 子标签切换 */}
      <div className="flex gap-1 border-b">
        {(['skills', 'uv', 'mcp'] as const).map((tab) => (
          <button
            key={tab}
            className={`px-4 py-2 text-sm border-b-2 transition-colors ${subTab === tab ? 'border-blue-600 text-blue-600 font-medium' : 'border-transparent text-gray-500 hover:text-gray-700'}`}
            onClick={() => setSubTab(tab)}
          >
            {tab === 'skills' ? '技能管理' : tab === 'uv' ? 'Python 环境 (uv)' : 'MCP 服务器'}
          </button>
        ))}
      </div>

      {/* ── Skills 子标签 ── */}
      {subTab === 'skills' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">已注册技能</h3>
            <button className="flex items-center gap-1 px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => { resetSkillForm(); setShowSkillForm(true); }}>
              <Plus size={14} /> 添加技能
            </button>
          </div>

          {showSkillForm && (
            <div className="p-4 border rounded-lg bg-gray-50 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <input className="border rounded px-3 py-2 text-sm" placeholder="技能标识（如 summary_gen）" value={skillForm.name} onChange={(e) => setSkillForm({ ...skillForm, name: e.target.value })} disabled={!!editSkillId} />
                <input className="border rounded px-3 py-2 text-sm" placeholder="显示名称（如 摘要生成）" value={skillForm.display_name ?? ''} onChange={(e) => setSkillForm({ ...skillForm, display_name: e.target.value || null })} />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <select className="border rounded px-3 py-2 text-sm" value={skillForm.skill_type} onChange={(e) => setSkillForm({ ...skillForm, skill_type: e.target.value })} disabled={!!editSkillId}>
                  <option value="builtin">内置</option>
                  <option value="python">Python</option>
                  <option value="external">外部</option>
                </select>
                <input className="border rounded px-3 py-2 text-sm" placeholder="来源（可选）" value={skillForm.source ?? ''} onChange={(e) => setSkillForm({ ...skillForm, source: e.target.value || null })} />
              </div>
              <input className="w-full border rounded px-3 py-2 text-sm" placeholder="描述（可选）" value={skillForm.description ?? ''} onChange={(e) => setSkillForm({ ...skillForm, description: e.target.value || null })} />
              <input className="w-full border rounded px-3 py-2 text-sm" placeholder="权限范围（可选）" value={skillForm.permission_scope ?? ''} onChange={(e) => setSkillForm({ ...skillForm, permission_scope: e.target.value || null })} />
              <div className="flex gap-2">
                <button className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={handleSubmitSkill}>{editSkillId ? '更新' : '创建'}</button>
                <button className="px-3 py-1.5 text-sm border rounded hover:bg-gray-100" onClick={resetSkillForm}>取消</button>
              </div>
            </div>
          )}

          {skillsQuery.data?.length ? (
            <div className="space-y-2">
              {skillsQuery.data.map((s) => (
                <div key={s.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
                  <div className="flex items-center gap-3">
                    {s.status === 'active' ? <CheckCircle2 size={16} className="text-green-500" /> : <XCircle size={16} className="text-gray-400" />}
                    <div>
                      <span className="font-medium">{s.display_name ?? s.name}</span>
                      <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 rounded">{s.skill_type}</span>
                      <span className={`ml-2 px-2 py-0.5 text-xs rounded ${s.health_status === 'healthy' ? 'bg-green-100 text-green-700' : s.health_status === 'unknown' ? 'bg-gray-100 text-gray-500' : 'bg-red-100 text-red-700'}`}>
                        {s.health_status}
                      </span>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button className="p-1.5 hover:bg-blue-100 rounded text-blue-600" title="健康检查" onClick={() => checkSkill.mutate(s.id)}><Activity size={14} /></button>
                    <button className="p-1.5 hover:bg-gray-200 rounded" title="编辑技能" onClick={() => startEditSkill(s)}><Edit2 size={14} /></button>
                    <button className="p-1.5 hover:bg-red-100 rounded text-red-500" title="删除技能" onClick={() => setDeleteTarget({ type: 'skill', id: s.id, name: s.display_name ?? s.name })}><Trash2 size={14} /></button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState title="暂无技能" description="添加内置或 Python 技能以扩展 AI 能力" icon={<Puzzle size={32} className="text-gray-400" />} />
          )}
        </div>
      )}

      {/* ── uv 子标签 ── */}
      {subTab === 'uv' && (
        <div className="space-y-4">
          <h3 className="text-lg font-semibold">Python 环境管理 (uv)</h3>
          <div className="p-4 border rounded-lg space-y-3">
            <div className="flex items-center gap-3">
              {uvHealth?.available ? <CheckCircle2 size={20} className="text-green-500" /> : <XCircle size={20} className="text-red-500" />}
              <div>
                <div className="font-medium">{uvHealth?.available ? 'uv 已安装' : 'uv 未安装'}</div>
                {uvHealth?.version && <div className="text-sm text-gray-500">版本: {uvHealth.version}</div>}
                {uvHealth?.path && <div className="text-sm text-gray-500 font-mono">{uvHealth.path}</div>}
                <div className="text-xs text-gray-400 mt-1">{uvHealth?.message}</div>
              </div>
            </div>
            <div className="flex gap-3">
              <button className="flex items-center gap-2 px-4 py-2 text-sm border rounded-lg hover:bg-gray-50" onClick={() => queryClient.invalidateQueries({ queryKey: ['uv-health'] })}>
                <RefreshCw size={14} /> 重新检测
              </button>
              {!uvHealth?.available && (
                <button className="flex items-center gap-2 px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => installUv.mutate()} disabled={installUv.isPending}>
                  <Download size={14} /> {installUv.isPending ? '安装中...' : '一键安装 uv'}
                </button>
              )}
            </div>
          </div>
          <p className="text-xs text-gray-400">uv 用于管理 Python 技能的隔离环境。安装来源: astral.sh（官方）。</p>
        </div>
      )}

      {/* ── MCP 子标签 ── */}
      {subTab === 'mcp' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">MCP 服务器</h3>
            <button className="flex items-center gap-1 px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => { resetMcpForm(); setShowMcpForm(true); }}>
              <Plus size={14} /> 添加 MCP 服务器
            </button>
          </div>

          {showMcpForm && (
            <div className="p-4 border rounded-lg bg-gray-50 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <input className="border rounded px-3 py-2 text-sm" placeholder="服务器标识" value={mcpForm.name} onChange={(e) => setMcpForm({ ...mcpForm, name: e.target.value })} disabled={!!editMcpId} />
                <input className="border rounded px-3 py-2 text-sm" placeholder="显示名称" value={mcpForm.display_name ?? ''} onChange={(e) => setMcpForm({ ...mcpForm, display_name: e.target.value || null })} />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <select className="border rounded px-3 py-2 text-sm" value={mcpForm.transport} onChange={(e) => setMcpForm({ ...mcpForm, transport: e.target.value })} disabled={!!editMcpId}>
                  <option value="stdio">Stdio（本地进程）</option>
                  <option value="http">HTTP（远程）</option>
                </select>
                <input className="border rounded px-3 py-2 text-sm" placeholder="启动命令（stdio 必填）" value={mcpForm.command ?? ''} onChange={(e) => setMcpForm({ ...mcpForm, command: e.target.value || null })} />
              </div>
              <input className="w-full border rounded px-3 py-2 text-sm" placeholder="启动参数 JSON（可选，如 [&quot;--port&quot;, &quot;8080&quot;]）" value={mcpForm.args_json ?? ''} onChange={(e) => setMcpForm({ ...mcpForm, args_json: e.target.value || null })} />
              <input className="w-full border rounded px-3 py-2 text-sm" placeholder="描述（可选）" value={mcpForm.description ?? ''} onChange={(e) => setMcpForm({ ...mcpForm, description: e.target.value || null })} />
              <div className="flex gap-2">
                <button className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700" onClick={handleSubmitMcp}>{editMcpId ? '更新' : '创建'}</button>
                <button className="px-3 py-1.5 text-sm border rounded hover:bg-gray-100" onClick={resetMcpForm}>取消</button>
              </div>
            </div>
          )}

          {mcpQuery.data?.length ? (
            <div className="space-y-2">
              {mcpQuery.data.map((m) => (
                <div key={m.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
                  <div className="flex items-center gap-3">
                    {m.enabled === 1 ? <CheckCircle2 size={16} className="text-green-500" /> : <XCircle size={16} className="text-gray-400" />}
                    <div>
                      <span className="font-medium">{m.display_name ?? m.name}</span>
                      <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 rounded">{m.transport}</span>
                      <span className={`ml-2 px-2 py-0.5 text-xs rounded ${m.health_status === 'healthy' ? 'bg-green-100 text-green-700' : m.health_status === 'unknown' ? 'bg-gray-100 text-gray-500' : 'bg-red-100 text-red-700'}`}>
                        {m.health_status}
                      </span>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button className="p-1.5 hover:bg-blue-100 rounded text-blue-600" title="健康检查" onClick={() => checkMcp.mutate(m.id)}><Activity size={14} /></button>
                    <button className="p-1.5 hover:bg-gray-200 rounded" title="编辑 MCP" onClick={() => startEditMcp(m)}><Edit2 size={14} /></button>
                    <button className="p-1.5 hover:bg-red-100 rounded text-red-500" title="删除 MCP" onClick={() => setDeleteTarget({ type: 'mcp', id: m.id, name: m.display_name ?? m.name })}><Trash2 size={14} /></button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState title="暂无 MCP 服务器" description="注册 MCP 服务器以扩展工具能力" icon={<Puzzle size={32} className="text-gray-400" />} />
          )}
        </div>
      )}

      {/* 删除确认 */}
      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除"
        message={`确定要删除「${deleteTarget?.name ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={() => {
          if (!deleteTarget) return;
          if (deleteTarget.type === 'skill') deleteSkillMut.mutate(deleteTarget.id);
          else deleteMcpMut.mutate(deleteTarget.id);
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════
   主组件 — 设置页面
   ═══════════════════════════════════════════════════════════════ */

/** 系统设置页面主组件 — 五标签页布局 */
export const SettingsPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState('ai');

  /** 根据当前激活标签渲染对应内容 */
  const renderContent = () => {
    switch (activeTab) {
      case 'ai': return <AiConfigTab />;
      case 'security': return <SecurityTab />;
      case 'template': return <TemplateTab />;
      case 'shortcut': return <ShortcutTab />;
      case 'skills': return <SkillsMcpTab />;
      default: return null;
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* 页面标题 */}
      <div className="flex items-center gap-2 mb-4">
        <Settings size={20} />
        <h2 className="text-xl font-bold">系统设置</h2>
      </div>

      {/* 标签导航栏 */}
      <div className="flex gap-1 border-b mb-6">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            className={`flex items-center gap-2 px-4 py-2.5 text-sm border-b-2 transition-colors ${
              activeTab === tab.key
                ? 'border-blue-600 text-blue-600 font-medium'
                : 'border-transparent text-gray-500 hover:text-gray-700'
            }`}
            onClick={() => setActiveTab(tab.key)}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>

      {/* 标签页内容区 */}
      <div className="flex-1 overflow-y-auto">
        {renderContent()}
      </div>
    </div>
  );
};
