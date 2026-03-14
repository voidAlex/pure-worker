/**
 * 系统设置页面
 * 包含 5 个主标签页：AI配置、安全隐私、模板导出、快捷键、Skills与MCP。
 */
import React, { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  commands,
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
  type AppError,
  type DeleteAiConfigInput,
  type DeleteAiParamPresetInput,
  type ActivateAiParamPresetInput,
  type ExportWorkspaceInput,
  type ArchiveWorkspaceInput,
  type EraseWorkspaceInput,
  type ListTemplateFilesInput,
  type DeleteTemplateFileInput,
  type DeleteGlobalShortcutInput,
  type DeleteSkillInput,
  type DeleteMcpServerInput,
  type CreateSkillEnvInput,
  type ProviderPreset,
  type ModelInfo,
} from '@/services/commandClient';
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
  Loader2,
  Check,
  Bot,
  Sparkles,
  Zap,
} from 'lucide-react';

/** 从 AppError 联合类型中提取错误信息字符串 */
const getErrorMessage = (err: AppError): string => {
  const values = Object.values(err as Record<string, string>);
  return values[0] ?? '未知错误';
};

/** 将 Tauri Result 统一解包为数据或抛错 */
const unwrapResult = <T,>(
  res: { status: 'ok'; data: T } | { status: 'error'; error: AppError },
): T => {
  if (res.status === 'ok') {
    return res.data;
  }
  throw new Error(getErrorMessage(res.error));
};

/** 将输入框文本转为可空 number */
const toNullableNumber = (value: string): number | null => {
  if (value.trim() === '') {
    return null;
  }
  const parsed = Number(value);
  return Number.isNaN(parsed) ? null : parsed;
};

type TabKey = 'ai' | 'security' | 'template' | 'shortcut' | 'skills';

const INITIAL_AI_FORM: CreateAiConfigInput = {
  provider_name: '',
  display_name: '',
  base_url: '',
  api_key: '',
  default_model: '',
  default_text_model: null,
  default_vision_model: null,
  default_tool_model: null,
  default_reasoning_model: null,
  is_active: null,
  config_json: null,
};

const INITIAL_PRESET_FORM: CreatePresetInput = {
  name: '',
  display_name: '',
  temperature: 0.7,
  top_p: null,
  max_tokens: null,
  is_default: null,
  is_active: null,
};

const INITIAL_TEMPLATE_FORM: CreateTemplateFileInput = {
  type: '',
  school_scope: null,
  version: null,
  file_path: '',
  enabled: 1,
};

const INITIAL_SHORTCUT_FORM: CreateGlobalShortcutInput = {
  action: '',
  key_combination: '',
  enabled: 1,
  description: null,
};

const INITIAL_SKILL_FORM: CreateSkillInput = {
  name: '',
  version: null,
  source: null,
  permission_scope: null,
  display_name: null,
  description: null,
  skill_type: 'builtin',
  env_path: null,
  config_json: null,
  license: null,
  compatibility: null,
  metadata_json: null,
  allowed_tools: null,
  body_content: null,
  entry_script: null,
};

const INITIAL_MCP_FORM: CreateMcpServerInput = {
  name: '',
  transport: 'stdio',
  command: null,
  args_json: null,
  env_json: null,
  permission_scope: null,
  display_name: null,
  description: null,
};

/** 供应商图标映射表：根据供应商名称返回对应的 Lucide 图标组件 */
const PROVIDER_ICON_MAP: Record<string, React.ReactNode> = {
  openai: <Bot size={24} className="text-green-600" />,
  anthropic: <Sparkles size={24} className="text-purple-600" />,
  deepseek: <Zap size={24} className="text-blue-600" />,
  qwen: <Cpu size={24} className="text-orange-600" />,
  gemini: <Activity size={24} className="text-red-500" />,
};

/** AI 配置标签页组件 — 供应商卡片选择 + 模型自动获取 */
const AiConfigTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  /* ---- 新建/编辑配置的表单状态 ---- */
  const [editingConfig, setEditingConfig] = useState<AiConfigSafe | null>(null);
  const [configForm, setConfigForm] = useState<CreateAiConfigInput>(INITIAL_AI_FORM);

  /* ---- 供应商选择与模型获取状态 ---- */
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [isCustom, setIsCustom] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [showConfigPanel, setShowConfigPanel] = useState(false);

  /* ---- 参数预设表单状态 ---- */
  const [showPresetForm, setShowPresetForm] = useState(false);
  const [editingPreset, setEditingPreset] = useState<AiParamPreset | null>(null);
  const [presetForm, setPresetForm] = useState<CreatePresetInput>(INITIAL_PRESET_FORM);

  /* ---- 删除确认状态 ---- */
  const [deleteTarget, setDeleteTarget] = useState<{
    type: 'config' | 'preset';
    id: string;
    name: string;
  } | null>(null);

  /* ---- 高级配置展开状态 ---- */
  const [showAdvanced, setShowAdvanced] = useState(false);

  /* ==================== 数据查询 ==================== */

  /** 查询供应商预设列表 */
  const presetsListQuery = useQuery({
    queryKey: ['settings', 'provider-presets'],
    queryFn: async () => unwrapResult(await commands.getProviderPresets()),
  });

  /** 查询已保存的 AI 配置列表 */
  const configsQuery = useQuery({
    queryKey: ['settings', 'ai-configs'],
    queryFn: async () => unwrapResult(await commands.listAiConfigs()),
  });

  /** 查询参数预设列表 */
  const presetsQuery = useQuery({
    queryKey: ['settings', 'ai-presets'],
    queryFn: async () => unwrapResult(await commands.listAiParamPresets()),
  });

  /* ==================== 变更操作 ==================== */

  /** 创建 AI 配置 */
  const createConfigMutation = useMutation({
    mutationFn: async (input: CreateAiConfigInput) =>
      unwrapResult(await commands.createAiConfig(input)),
    onSuccess: () => {
      success('AI 配置已创建');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-configs'] });
      resetConfigPanel();
    },
    onError: (err: Error) => error(err.message),
  });

  /** 更新 AI 配置 */
  const updateConfigMutation = useMutation({
    mutationFn: async (input: UpdateAiConfigInput) =>
      unwrapResult(await commands.updateAiConfig(input)),
    onSuccess: () => {
      success('AI 配置已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-configs'] });
      resetConfigPanel();
    },
    onError: (err: Error) => error(err.message),
  });

  /** 删除 AI 配置 */
  const deleteConfigMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteAiConfigInput = { id };
      return unwrapResult(await commands.deleteAiConfig(input));
    },
    onSuccess: () => {
      success('AI 配置已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-configs'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /** 创建参数预设 */
  const createPresetMutation = useMutation({
    mutationFn: async (input: CreatePresetInput) =>
      unwrapResult(await commands.createAiParamPreset(input)),
    onSuccess: () => {
      success('参数预设已创建');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-presets'] });
      setShowPresetForm(false);
      setEditingPreset(null);
      setPresetForm(INITIAL_PRESET_FORM);
    },
    onError: (err: Error) => error(err.message),
  });

  /** 更新参数预设 */
  const updatePresetMutation = useMutation({
    mutationFn: async (input: UpdatePresetInput) =>
      unwrapResult(await commands.updateAiParamPreset(input)),
    onSuccess: () => {
      success('参数预设已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-presets'] });
      setShowPresetForm(false);
      setEditingPreset(null);
      setPresetForm(INITIAL_PRESET_FORM);
    },
    onError: (err: Error) => error(err.message),
  });

  /** 删除参数预设 */
  const deletePresetMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteAiParamPresetInput = { id };
      return unwrapResult(await commands.deleteAiParamPreset(input));
    },
    onSuccess: () => {
      success('参数预设已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-presets'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /** 激活参数预设 */
  const activatePresetMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: ActivateAiParamPresetInput = { id };
      return unwrapResult(await commands.activateAiParamPreset(input));
    },
    onSuccess: () => {
      success('预设已激活');
      queryClient.invalidateQueries({ queryKey: ['settings', 'ai-presets'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /* ==================== 操作函数 ==================== */

  /** 重置配置面板到初始状态 */
  const resetConfigPanel = () => {
    setShowConfigPanel(false);
    setEditingConfig(null);
    setConfigForm(INITIAL_AI_FORM);
    setSelectedPreset(null);
    setIsCustom(false);
    setModels([]);
    setShowApiKey(false);
    setShowAdvanced(false);
  };

  /** 选择内置供应商预设卡片 */
  const selectProviderPreset = (preset: ProviderPreset) => {
    setSelectedPreset(preset);
    setIsCustom(false);
    setModels([]);
    setConfigForm((prev) => ({
      ...prev,
      provider_name: preset.name,
      display_name: preset.display_name,
      base_url: preset.base_url,
      default_model: '',
    }));
    setShowConfigPanel(true);
  };

  /** 选择自定义供应商 */
  const selectCustomProvider = () => {
    setSelectedPreset(null);
    setIsCustom(true);
    setModels([]);
    setConfigForm({ ...INITIAL_AI_FORM });
    setShowConfigPanel(true);
  };

  /** 打开已有配置编辑 */
  const openEditConfigForm = (item: AiConfigSafe) => {
    setEditingConfig(item);
    setSelectedPreset(null);
    setIsCustom(true);
    setModels([]);
    setConfigForm({
      provider_name: item.provider_name,
      display_name: item.display_name,
      base_url: item.base_url,
      api_key: '',
      default_model: item.default_model,
      default_text_model: item.default_text_model,
      default_vision_model: item.default_vision_model,
      default_tool_model: item.default_tool_model,
      default_reasoning_model: item.default_reasoning_model,
      is_active: item.is_active === 1,
      config_json: item.config_json,
    });
    setShowConfigPanel(true);
  };

  /** 通过 API 获取供应商的模型列表 */
  const handleFetchModels = async () => {
    const providerName = configForm.provider_name.trim();
    const baseUrl = configForm.base_url.trim();
    const apiKey = configForm.api_key.trim();
    if (!providerName || !baseUrl || !apiKey) {
      error('请先填写供应商名称、接口地址和 API Key');
      return;
    }
    setFetchingModels(true);
    try {
      const result = await commands.fetchProviderModels(providerName, baseUrl, apiKey);
      const modelList = unwrapResult(result);
      setModels(modelList);
      if (modelList.length > 0) {
        /* 自动选中第一个模型作为默认模型（若当前未设置） */
        if (!configForm.default_model) {
          setConfigForm((prev) => ({ ...prev, default_model: modelList[0].id }));
        }
        success(`获取到 ${modelList.length} 个模型`);
      } else {
        error('未获取到可用模型，请检查 API Key 是否正确');
      }
    } catch (err) {
      error(err instanceof Error ? err.message : '获取模型列表失败');
      setModels([]);
    } finally {
      setFetchingModels(false);
    }
  };

  /** 选中一个模型作为默认模型 */
  const selectModel = (modelId: string) => {
    setConfigForm((prev) => ({ ...prev, default_model: modelId }));
  };

  /** 提交 AI 配置表单（创建或更新） */
  const submitConfigForm = () => {
    if (!configForm.provider_name.trim() || !configForm.base_url.trim()) {
      error('供应商名称和接口地址不能为空');
      return;
    }
    if (!editingConfig && !configForm.api_key.trim()) {
      error('API Key 不能为空');
      return;
    }
    if (editingConfig) {
      const input: UpdateAiConfigInput = {
        id: editingConfig.id,
        display_name: configForm.display_name.trim() === '' ? null : configForm.display_name,
        base_url: configForm.base_url.trim() === '' ? null : configForm.base_url,
        api_key: configForm.api_key.trim() === '' ? null : configForm.api_key,
        default_model: configForm.default_model.trim() === '' ? null : configForm.default_model,
        default_text_model: configForm.default_text_model?.trim() === '' ? null : configForm.default_text_model,
        default_vision_model: configForm.default_vision_model?.trim() === '' ? null : configForm.default_vision_model,
        default_tool_model: configForm.default_tool_model?.trim() === '' ? null : configForm.default_tool_model,
        default_reasoning_model: configForm.default_reasoning_model?.trim() === '' ? null : configForm.default_reasoning_model,
        is_active: configForm.is_active,
        config_json: configForm.config_json,
      };
      updateConfigMutation.mutate(input);
      return;
    }
    createConfigMutation.mutate(configForm);
  };

  /** 打开参数预设创建表单 */
  const openCreatePresetForm = () => {
    setEditingPreset(null);
    setPresetForm(INITIAL_PRESET_FORM);
    setShowPresetForm(true);
  };

  /** 打开参数预设编辑表单 */
  const openEditPresetForm = (item: AiParamPreset) => {
    setEditingPreset(item);
    setPresetForm({
      name: item.name,
      display_name: item.display_name,
      temperature: item.temperature,
      top_p: item.top_p,
      max_tokens: item.max_tokens,
      is_default: item.is_default === 1,
      is_active: item.is_active === 1,
    });
    setShowPresetForm(true);
  };

  /** 提交参数预设表单 */
  const submitPresetForm = () => {
    if (editingPreset) {
      const input: UpdatePresetInput = {
        id: editingPreset.id,
        name: presetForm.name.trim() === '' ? null : presetForm.name,
        display_name: presetForm.display_name.trim() === '' ? null : presetForm.display_name,
        temperature: presetForm.temperature,
        top_p: presetForm.top_p,
        max_tokens: presetForm.max_tokens,
        is_default: presetForm.is_default,
        is_active: presetForm.is_active,
      };
      updatePresetMutation.mutate(input);
      return;
    }
    createPresetMutation.mutate(presetForm);
  };

  /** 执行删除动作 */
  const confirmDelete = () => {
    if (!deleteTarget) return;
    if (deleteTarget.type === 'config') {
      deleteConfigMutation.mutate(deleteTarget.id);
    } else {
      deletePresetMutation.mutate(deleteTarget.id);
    }
    setDeleteTarget(null);
  };

  /* ==================== 渲染 ==================== */

  return (
    <div className="space-y-8">
      {/* ===== 第一区：新建 AI 服务配置 ===== */}
      <section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="mb-4 flex items-center gap-2 text-lg font-semibold text-gray-900">
          <Bot size={20} className="text-blue-600" />
          新建 AI 服务配置
        </h3>
        <p className="mb-4 text-sm text-gray-500">
          选择一个供应商，填入 API Key 后即可自动获取模型列表
        </p>

        {/* 供应商卡片网格 */}
        <div className="mb-4 grid grid-cols-2 gap-3 sm:grid-cols-3">
          {presetsListQuery.data?.map((preset) => (
            <button
              key={preset.name}
              onClick={() => selectProviderPreset(preset)}
              className={`flex flex-col items-center gap-2 rounded-xl border-2 p-4 transition-all hover:shadow-md ${
                selectedPreset?.name === preset.name && !isCustom
                  ? 'border-blue-500 bg-blue-50 shadow-md'
                  : 'border-gray-200 bg-white hover:border-gray-300'
              }`}
            >
              {PROVIDER_ICON_MAP[preset.name] ?? <Bot size={24} className="text-gray-500" />}
              <span className="text-sm font-medium text-gray-800">{preset.display_name}</span>
            </button>
          ))}
          {/* 自定义供应商卡片 */}
          <button
            onClick={selectCustomProvider}
            className={`flex flex-col items-center gap-2 rounded-xl border-2 border-dashed p-4 transition-all hover:shadow-md ${
              isCustom
                ? 'border-blue-500 bg-blue-50 shadow-md'
                : 'border-gray-300 bg-gray-50 hover:border-gray-400'
            }`}
          >
            <Plus size={24} className="text-gray-500" />
            <span className="text-sm font-medium text-gray-600">自定义</span>
          </button>
        </div>

        {/* 配置表单面板 — 选择供应商后展开 */}
        {showConfigPanel && (
          <div className="space-y-4 rounded-lg border border-gray-200 bg-gray-50 p-5">
            {/* 自定义供应商需要输入名称和地址 */}
            {isCustom && !editingConfig && (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <div>
                  <label className="mb-1 block text-xs font-medium text-gray-600">供应商名称</label>
                  <input
                    className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                    placeholder="例如: openai"
                    value={configForm.provider_name}
                    onChange={(e) =>
                      setConfigForm((prev) => ({ ...prev, provider_name: e.target.value }))
                    }
                  />
                </div>
                <div>
                  <label className="mb-1 block text-xs font-medium text-gray-600">显示名称</label>
                  <input
                    className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                    placeholder="例如: OpenAI"
                    value={configForm.display_name}
                    onChange={(e) =>
                      setConfigForm((prev) => ({ ...prev, display_name: e.target.value }))
                    }
                  />
                </div>
                <div className="sm:col-span-2">
                  <label className="mb-1 block text-xs font-medium text-gray-600">
                    接口地址 (Base URL)
                  </label>
                  <input
                    className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                    placeholder="https://api.example.com"
                    value={configForm.base_url}
                    onChange={(e) =>
                      setConfigForm((prev) => ({ ...prev, base_url: e.target.value }))
                    }
                  />
                </div>
              </div>
            )}

            {/* 编辑已有配置时显示供应商信息（只读） */}
            {editingConfig && (
              <div className="flex items-center gap-3 rounded-lg bg-white p-3">
                {PROVIDER_ICON_MAP[editingConfig.provider_name] ?? (
                  <Bot size={20} className="text-gray-500" />
                )}
                <div>
                  <div className="text-sm font-medium text-gray-800">
                    {editingConfig.display_name}
                  </div>
                  <div className="text-xs text-gray-500">
                    {editingConfig.provider_name} — {editingConfig.base_url}
                  </div>
                </div>
              </div>
            )}

            {/* API Key 输入 + 获取模型按钮 */}
            <div>
              <label className="mb-1 block text-xs font-medium text-gray-600">
                API Key {editingConfig ? '（留空则不修改）' : ''}
              </label>
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <input
                    className="w-full rounded-lg border border-gray-300 px-3 py-2 pr-10 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                    type={showApiKey ? 'text' : 'password'}
                    placeholder="sk-..."
                    value={configForm.api_key}
                    onChange={(e) =>
                      setConfigForm((prev) => ({ ...prev, api_key: e.target.value }))
                    }
                  />
                  <button
                    type="button"
                    onClick={() => setShowApiKey((v) => !v)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-gray-400 hover:text-gray-600"
                    title={showApiKey ? '隐藏密钥' : '显示密钥'}
                  >
                    {showApiKey ? <EyeOff size={16} /> : <Eye size={16} />}
                  </button>
                </div>
                <button
                  onClick={handleFetchModels}
                  disabled={fetchingModels}
                  className="flex items-center gap-1.5 whitespace-nowrap rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-700 disabled:opacity-50"
                >
                  {fetchingModels ? (
                    <Loader2 size={16} className="animate-spin" />
                  ) : (
                    <RefreshCw size={16} />
                  )}
                  获取模型列表
                </button>
              </div>
            </div>

            {/* 模型列表 */}
            {models.length > 0 && (
              <div>
                <label className="mb-2 block text-xs font-medium text-gray-600">
                  可用模型（点击选择默认模型）
                </label>
                <div className="max-h-60 space-y-1 overflow-y-auto rounded-lg border border-gray-200 bg-white p-2">
                  {models.map((model) => (
                    <button
                      key={model.id}
                      onClick={() => selectModel(model.id)}
                      className={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                        configForm.default_model === model.id
                          ? 'bg-blue-50 text-blue-800 ring-1 ring-blue-300'
                          : 'hover:bg-gray-50'
                      }`}
                    >
                      <div className="flex items-center gap-2">
                        {configForm.default_model === model.id && (
                          <Check size={14} className="text-blue-600" />
                        )}
                        <span
                          className={configForm.default_model === model.id ? 'font-medium' : ''}
                        >
                          {model.name}
                        </span>
                      </div>
                      <span
                        className={`rounded-full px-2 py-0.5 text-xs font-medium ${
                          model.is_vision
                            ? 'bg-green-100 text-green-700'
                            : 'bg-blue-100 text-blue-700'
                        }`}
                      >
                        {model.is_vision ? '多模态' : '文本'}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* 高级选项（可折叠） */}
            <div>
              <button
                type="button"
                onClick={() => setShowAdvanced((v) => !v)}
                className="flex items-center gap-1 text-xs text-gray-500 hover:text-gray-700"
              >
                <Settings size={12} />
                {showAdvanced ? '收起高级选项' : '展开高级选项'}
              </button>
              {showAdvanced && (
                <div className="mt-2 space-y-3">
                  {isCustom && editingConfig && (
                    <>
                      <div>
                        <label className="mb-1 block text-xs font-medium text-gray-600">
                          显示名称
                        </label>
                        <input
                          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                          value={configForm.display_name}
                          onChange={(e) =>
                            setConfigForm((prev) => ({ ...prev, display_name: e.target.value }))
                          }
                        />
                      </div>
                      <div>
                        <label className="mb-1 block text-xs font-medium text-gray-600">
                          接口地址
                        </label>
                        <input
                          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                          value={configForm.base_url}
                          onChange={(e) =>
                            setConfigForm((prev) => ({ ...prev, base_url: e.target.value }))
                          }
                        />
                      </div>
                    </>
                  )}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-gray-600">
                      默认模型（手动输入）
                    </label>
                    <input
                      className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                      placeholder="gpt-4o"
                      value={configForm.default_model}
                      onChange={(e) =>
                        setConfigForm((prev) => ({ ...prev, default_model: e.target.value }))
                      }
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs font-medium text-gray-600">
                      配置 JSON（可选）
                    </label>
                    <input
                      className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                      placeholder='{"key": "value"}'
                      value={configForm.config_json ?? ''}
                      onChange={(e) =>
                        setConfigForm((prev) => ({
                          ...prev,
                          config_json: e.target.value.trim() === '' ? null : e.target.value,
                        }))
                      }
                    />
                  </div>
                </div>
              )}
            </div>

            {/* 激活开关 + 保存/取消按钮 */}
            <div className="flex items-center justify-between border-t border-gray-200 pt-4">
              <label className="flex items-center gap-2 text-sm text-gray-700">
                <input
                  type="checkbox"
                  className="rounded"
                  checked={configForm.is_active === true}
                  onChange={(e) =>
                    setConfigForm((prev) => ({ ...prev, is_active: e.target.checked }))
                  }
                />
                设为激活
              </label>
              <div className="flex gap-2">
                <button
                  onClick={resetConfigPanel}
                  className="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-100"
                >
                  取消
                </button>
                <button
                  onClick={submitConfigForm}
                  disabled={createConfigMutation.isPending || updateConfigMutation.isPending}
                  className="flex items-center gap-1.5 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-700 disabled:opacity-50"
                >
                  {(createConfigMutation.isPending || updateConfigMutation.isPending) && (
                    <Loader2 size={14} className="animate-spin" />
                  )}
                  {editingConfig ? '更新配置' : '保存配置'}
                </button>
              </div>
            </div>
          </div>
        )}
      </section>

      {/* ===== 第二区：已保存的 AI 配置列表 ===== */}
      <section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="mb-4 flex items-center gap-2 text-lg font-semibold text-gray-900">
          <CheckCircle2 size={20} className="text-green-600" />
          已保存的服务配置
        </h3>
        {configsQuery.data && configsQuery.data.length > 0 ? (
          <div className="space-y-3">
            {configsQuery.data.map((item) => (
              <div
                key={item.id}
                className="flex items-center justify-between rounded-xl border border-gray-200 bg-gray-50 p-4 transition-colors hover:bg-gray-100"
              >
                <div className="flex items-center gap-3">
                  {PROVIDER_ICON_MAP[item.provider_name] ?? (
                    <Bot size={20} className="text-gray-400" />
                  )}
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-gray-900">{item.display_name}</span>
                      {item.is_active === 1 && (
                        <span className="rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700">
                          激活
                        </span>
                      )}
                    </div>
                    <div className="mt-0.5 text-xs text-gray-500">
                      {item.provider_name} · 默认模型: {item.default_model || '未设置'}
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  <button
                    className="flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs text-gray-600 hover:bg-gray-200"
                    onClick={() => openEditConfigForm(item)}
                    title="编辑配置"
                  >
                    <Edit2 size={14} />
                    编辑
                  </button>
                  <button
                    className="flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs text-red-500 hover:bg-red-50"
                    onClick={() =>
                      setDeleteTarget({ type: 'config', id: item.id, name: item.display_name })
                    }
                    title="删除配置"
                  >
                    <Trash2 size={14} />
                    删除
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            title="暂无 AI 配置"
            description="请在上方选择供应商并添加配置"
            icon={<Cpu size={32} className="text-gray-400" />}
          />
        )}
      </section>

      {/* ===== 第三区：参数预设 ===== */}
      <section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="flex items-center gap-2 text-lg font-semibold text-gray-900">
            <Settings size={20} className="text-gray-600" />
            参数预设
          </h3>
          <button
            className="flex items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-blue-700"
            onClick={openCreatePresetForm}
          >
            <Plus size={14} /> 添加预设
          </button>
        </div>

        {showPresetForm && (
          <div className="mb-4 space-y-3 rounded-lg border border-gray-200 bg-gray-50 p-4">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <div>
                <label className="mb-1 block text-xs font-medium text-gray-600">预设标识</label>
                <input
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  placeholder="例如: creative"
                  value={presetForm.name}
                  onChange={(e) => setPresetForm((prev) => ({ ...prev, name: e.target.value }))}
                />
              </div>
              <div>
                <label className="mb-1 block text-xs font-medium text-gray-600">显示名称</label>
                <input
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  placeholder="例如: 创意模式"
                  value={presetForm.display_name}
                  onChange={(e) =>
                    setPresetForm((prev) => ({ ...prev, display_name: e.target.value }))
                  }
                />
              </div>
              <div>
                <label className="mb-1 block text-xs font-medium text-gray-600">Temperature</label>
                <input
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  type="number"
                  step={0.1}
                  min={0}
                  max={2}
                  value={presetForm.temperature}
                  onChange={(e) =>
                    setPresetForm((prev) => ({
                      ...prev,
                      temperature: Number.isNaN(Number(e.target.value))
                        ? 0.7
                        : Number(e.target.value),
                    }))
                  }
                />
              </div>
              <div>
                <label className="mb-1 block text-xs font-medium text-gray-600">Top P</label>
                <input
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  type="number"
                  step={0.1}
                  min={0}
                  max={1}
                  placeholder="可选"
                  value={presetForm.top_p ?? ''}
                  onChange={(e) =>
                    setPresetForm((prev) => ({ ...prev, top_p: toNullableNumber(e.target.value) }))
                  }
                />
              </div>
              <div>
                <label className="mb-1 block text-xs font-medium text-gray-600">Max Tokens</label>
                <input
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                  type="number"
                  min={1}
                  placeholder="可选"
                  value={presetForm.max_tokens ?? ''}
                  onChange={(e) =>
                    setPresetForm((prev) => ({
                      ...prev,
                      max_tokens: toNullableNumber(e.target.value),
                    }))
                  }
                />
              </div>
            </div>
            <div className="flex gap-4">
              <label className="flex items-center gap-2 text-sm text-gray-700">
                <input
                  type="checkbox"
                  className="rounded"
                  checked={presetForm.is_default === true}
                  onChange={(e) =>
                    setPresetForm((prev) => ({ ...prev, is_default: e.target.checked }))
                  }
                />
                默认
              </label>
              <label className="flex items-center gap-2 text-sm text-gray-700">
                <input
                  type="checkbox"
                  className="rounded"
                  checked={presetForm.is_active === true}
                  onChange={(e) =>
                    setPresetForm((prev) => ({ ...prev, is_active: e.target.checked }))
                  }
                />
                激活
              </label>
            </div>
            <div className="flex gap-2">
              <button
                className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-700"
                onClick={submitPresetForm}
              >
                {editingPreset ? '更新预设' : '创建预设'}
              </button>
              <button
                className="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-100"
                onClick={() => {
                  setShowPresetForm(false);
                  setEditingPreset(null);
                }}
              >
                取消
              </button>
            </div>
          </div>
        )}

        {presetsQuery.data && presetsQuery.data.length > 0 ? (
          <div className="space-y-2">
            {presetsQuery.data.map((item) => (
              <div
                key={item.id}
                className="flex items-center justify-between rounded-xl border border-gray-200 bg-gray-50 p-3 transition-colors hover:bg-gray-100"
              >
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-gray-900">{item.display_name}</span>
                    {item.is_active === 1 && (
                      <span className="rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                        激活
                      </span>
                    )}
                    {item.is_default === 1 && (
                      <span className="rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700">
                        默认
                      </span>
                    )}
                  </div>
                  <div className="mt-0.5 text-xs text-gray-500">
                    {item.name} · T={item.temperature} / P={item.top_p ?? '-'} / MaxTok=
                    {item.max_tokens ?? '-'}
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  {item.is_active !== 1 && (
                    <button
                      className="flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs text-blue-600 hover:bg-blue-50"
                      onClick={() => activatePresetMutation.mutate(item.id)}
                      title="激活预设"
                    >
                      <Play size={14} />
                      激活
                    </button>
                  )}
                  <button
                    className="flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs text-gray-600 hover:bg-gray-200"
                    onClick={() => openEditPresetForm(item)}
                    title="编辑预设"
                  >
                    <Edit2 size={14} />
                    编辑
                  </button>
                  <button
                    className="flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs text-red-500 hover:bg-red-50"
                    onClick={() =>
                      setDeleteTarget({ type: 'preset', id: item.id, name: item.display_name })
                    }
                    title="删除预设"
                  >
                    <Trash2 size={14} />
                    删除
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            title="暂无参数预设"
            description="请创建参数预设用于快速切换"
            icon={<Settings size={32} className="text-gray-400" />}
          />
        )}
      </section>

      {/* 删除确认弹窗 */}
      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除"
        message={`确定要删除「${deleteTarget?.name ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/** 安全隐私标签页组件 */
const SecurityTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();
  const [confirmAction, setConfirmAction] = useState<'export' | 'archive' | 'erase' | null>(null);

  const statsQuery = useQuery({
    queryKey: ['settings', 'storage-stats'],
    queryFn: async (): Promise<StorageStats> => unwrapResult(await commands.getStorageStats()),
  });

  const desensitizeQuery = useQuery({
    queryKey: ['settings', 'desensitize_enabled'],
    queryFn: async () => {
      const res = await commands.getSetting('desensitize_enabled');
      if (res.status === 'ok') {
        return res.data.value === 'true';
      }
      return true;
    },
  });

  const toggleDesensitizeMutation = useMutation({
    mutationFn: async (enabled: boolean) =>
      unwrapResult(
        await commands.updateSetting(
          'desensitize_enabled',
          String(enabled),
          'security',
          '外发前脱敏开关',
        ),
      ),
    onSuccess: () => {
      success('脱敏设置已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'desensitize_enabled'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const exportMutation = useMutation({
    mutationFn: async () => {
      const input: ExportWorkspaceInput = { output_path: 'workspace_export.zip', approved: true };
      return unwrapResult(await commands.exportWorkspace(input));
    },
    onSuccess: (data) => {
      success(`导出成功：${data.output_path}`);
      queryClient.invalidateQueries({ queryKey: ['settings', 'storage-stats'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const archiveMutation = useMutation({
    mutationFn: async () => {
      const input: ArchiveWorkspaceInput = { archive_name: null };
      return unwrapResult(await commands.archiveWorkspace(input));
    },
    onSuccess: (data) => {
      success(`归档成功：${data.archive_path}`);
      queryClient.invalidateQueries({ queryKey: ['settings', 'storage-stats'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const eraseMutation = useMutation({
    mutationFn: async () => {
      const input: EraseWorkspaceInput = { approved: true };
      return unwrapResult(await commands.eraseWorkspace(input));
    },
    onSuccess: () => {
      success('工作区数据已清除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'storage-stats'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /** 执行高危确认动作 */
  const onConfirmAction = () => {
    if (confirmAction === 'export') {
      exportMutation.mutate();
    }
    if (confirmAction === 'archive') {
      archiveMutation.mutate();
    }
    if (confirmAction === 'erase') {
      eraseMutation.mutate();
    }
    setConfirmAction(null);
  };

  return (
    <div className="space-y-6">
      <section>
        <h3 className="mb-3 text-lg font-semibold">存储统计</h3>
        {statsQuery.data ? (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
            <div className="rounded-lg border p-4">
              <div className="text-sm text-gray-500">工作区路径</div>
              <div
                className="mt-1 truncate font-mono text-sm"
                title={statsQuery.data.workspace_path}
              >
                {statsQuery.data.workspace_path}
              </div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-sm text-gray-500">文件总量</div>
              <div className="mt-1 text-lg font-semibold">{statsQuery.data.total_files}</div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-sm text-gray-500">总大小 / 归档数</div>
              <div className="mt-1 text-lg font-semibold">
                {statsQuery.data.total_size_display} / {statsQuery.data.archive_count}
              </div>
            </div>
          </div>
        ) : (
          <div className="text-sm text-gray-500">加载中...</div>
        )}
      </section>

      <section>
        <h3 className="mb-3 text-lg font-semibold">发送前脱敏</h3>
        <div className="flex items-center gap-3 rounded-lg border p-4">
          <button
            className={`relative h-6 w-12 rounded-full ${desensitizeQuery.data ? 'bg-blue-600' : 'bg-gray-300'}`}
            onClick={() => toggleDesensitizeMutation.mutate(!(desensitizeQuery.data ?? true))}
          >
            <span
              className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition-transform ${
                desensitizeQuery.data ? 'translate-x-6' : 'translate-x-0.5'
              }`}
            />
          </button>
          <div>
            <div className="flex items-center gap-2 text-sm font-medium">
              {desensitizeQuery.data ? (
                <Eye size={16} className="text-blue-600" />
              ) : (
                <EyeOff size={16} className="text-gray-400" />
              )}
              {desensitizeQuery.data ? '脱敏已开启' : '脱敏已关闭'}
            </div>
            <div className="mt-1 text-xs text-gray-500">
              开启后，外发内容将优先进行敏感信息脱敏。
            </div>
          </div>
        </div>
      </section>

      <section>
        <h3 className="mb-3 text-lg font-semibold">数据生命周期</h3>
        <div className="flex flex-wrap gap-3">
          <button
            className="flex items-center gap-2 rounded-lg border px-4 py-2 text-sm hover:bg-gray-50"
            onClick={() => setConfirmAction('export')}
          >
            <Download size={16} /> 导出工作区
          </button>
          <button
            className="flex items-center gap-2 rounded-lg border px-4 py-2 text-sm hover:bg-gray-50"
            onClick={() => setConfirmAction('archive')}
          >
            <Archive size={16} /> 归档工作区
          </button>
          <button
            className="flex items-center gap-2 rounded-lg border border-red-200 px-4 py-2 text-sm text-red-600 hover:bg-red-50"
            onClick={() => setConfirmAction('erase')}
          >
            <AlertTriangle size={16} /> 清除数据
          </button>
        </div>
      </section>

      <ConfirmDialog
        isOpen={confirmAction !== null}
        title={confirmAction === 'erase' ? '⚠️ 确认清除所有数据' : '确认操作'}
        message={
          confirmAction === 'erase'
            ? '此操作将永久删除工作区数据，且不可恢复。'
            : confirmAction === 'archive'
              ? '归档会打包当前工作区数据。'
              : '导出会生成工作区备份文件。'
        }
        confirmText={confirmAction === 'erase' ? '确认清除' : '确认'}
        isDestructive={confirmAction === 'erase'}
        onConfirm={onConfirmAction}
        onCancel={() => setConfirmAction(null)}
      />
    </div>
  );
};

/** 模板导出标签页组件 */
const TemplateTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  const [showForm, setShowForm] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<TemplateFile | null>(null);
  const [form, setForm] = useState<CreateTemplateFileInput>(INITIAL_TEMPLATE_FORM);
  const [deleteTarget, setDeleteTarget] = useState<TemplateFile | null>(null);

  const templatesQuery = useQuery({
    queryKey: ['settings', 'templates'],
    queryFn: async () => {
      const input: ListTemplateFilesInput = { type: null, enabled: null };
      return unwrapResult(await commands.listTemplateFiles(input));
    },
  });

  const exportFormatQuery = useQuery({
    queryKey: ['settings', 'default_export_format'],
    queryFn: async () => {
      const res = await commands.getSetting('default_export_format');
      if (res.status === 'ok') {
        return res.data.value;
      }
      return 'docx';
    },
  });

  const updateExportFormatMutation = useMutation({
    mutationFn: async (format: string) =>
      unwrapResult(
        await commands.updateSetting('default_export_format', format, 'export', '默认导出格式'),
      ),
    onSuccess: () => {
      success('默认导出格式已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'default_export_format'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const createTemplateMutation = useMutation({
    mutationFn: async (input: CreateTemplateFileInput) =>
      unwrapResult(await commands.createTemplateFile(input)),
    onSuccess: () => {
      success('模板已创建');
      queryClient.invalidateQueries({ queryKey: ['settings', 'templates'] });
      setShowForm(false);
      setForm(INITIAL_TEMPLATE_FORM);
      setEditingTemplate(null);
    },
    onError: (err: Error) => error(err.message),
  });

  const updateTemplateMutation = useMutation({
    mutationFn: async (payload: { id: string; input: CreateTemplateFileInput }) =>
      unwrapResult(
        await commands.updateTemplateFile({
          id: payload.id,
          type: payload.input.type.trim() === '' ? null : payload.input.type,
          school_scope: payload.input.school_scope,
          version: payload.input.version,
          file_path: payload.input.file_path.trim() === '' ? null : payload.input.file_path,
          enabled: payload.input.enabled,
        }),
      ),
    onSuccess: () => {
      success('模板已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'templates'] });
      setShowForm(false);
      setForm(INITIAL_TEMPLATE_FORM);
      setEditingTemplate(null);
    },
    onError: (err: Error) => error(err.message),
  });

  const deleteTemplateMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteTemplateFileInput = { id };
      return unwrapResult(await commands.deleteTemplateFile(input));
    },
    onSuccess: () => {
      success('模板已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'templates'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /** 打开模板创建表单 */
  const openCreateTemplateForm = () => {
    setEditingTemplate(null);
    setForm(INITIAL_TEMPLATE_FORM);
    setShowForm(true);
  };

  /** 打开模板编辑表单 */
  const openEditTemplateForm = (item: TemplateFile) => {
    setEditingTemplate(item);
    setForm({
      type: item.type,
      school_scope: item.school_scope,
      version: item.version,
      file_path: item.file_path,
      enabled: item.enabled,
    });
    setShowForm(true);
  };

  /** 提交模板表单 */
  const submitTemplateForm = () => {
    if (editingTemplate) {
      updateTemplateMutation.mutate({ id: editingTemplate.id, input: form });
      return;
    }
    createTemplateMutation.mutate(form);
  };

  return (
    <div className="space-y-6">
      <section>
        <h3 className="mb-3 text-lg font-semibold">默认导出格式</h3>
        <div className="flex gap-2">
          {['docx', 'pdf', 'txt', 'markdown'].map((format) => (
            <button
              key={format}
              className={`rounded border px-3 py-1.5 text-sm ${
                exportFormatQuery.data === format
                  ? 'border-blue-600 bg-blue-600 text-white'
                  : 'hover:bg-gray-50'
              }`}
              onClick={() => updateExportFormatMutation.mutate(format)}
            >
              {format.toUpperCase()}
            </button>
          ))}
        </div>
      </section>

      <section>
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-lg font-semibold">模板文件</h3>
          <button
            className="flex items-center gap-1 rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
            onClick={openCreateTemplateForm}
          >
            <Plus size={14} /> 添加模板
          </button>
        </div>

        {showForm && (
          <div className="mb-4 space-y-3 rounded-lg border bg-gray-50 p-4">
            <div className="grid grid-cols-2 gap-3">
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="type"
                value={form.type}
                onChange={(e) => setForm((prev) => ({ ...prev, type: e.target.value }))}
              />
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="file_path"
                value={form.file_path}
                onChange={(e) => setForm((prev) => ({ ...prev, file_path: e.target.value }))}
              />
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="school_scope（可选）"
                value={form.school_scope ?? ''}
                onChange={(e) =>
                  setForm((prev) => ({
                    ...prev,
                    school_scope: e.target.value.trim() === '' ? null : e.target.value,
                  }))
                }
              />
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="version（可选）"
                value={form.version ?? ''}
                onChange={(e) =>
                  setForm((prev) => ({
                    ...prev,
                    version: e.target.value.trim() === '' ? null : e.target.value,
                  }))
                }
              />
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={(form.enabled ?? 0) === 1}
                onChange={(e) =>
                  setForm((prev) => ({ ...prev, enabled: e.target.checked ? 1 : 0 }))
                }
              />
              启用模板
            </label>
            <div className="flex gap-2">
              <button
                className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                onClick={submitTemplateForm}
              >
                {editingTemplate ? '更新' : '创建'}
              </button>
              <button
                className="rounded border px-3 py-1.5 text-sm hover:bg-gray-100"
                onClick={() => {
                  setShowForm(false);
                  setEditingTemplate(null);
                }}
              >
                取消
              </button>
            </div>
          </div>
        )}

        {templatesQuery.data && templatesQuery.data.length > 0 ? (
          <div className="space-y-2">
            {templatesQuery.data.map((item) => (
              <div
                key={item.id}
                className="flex items-center justify-between rounded-lg border p-3 hover:bg-gray-50"
              >
                <div>
                  <div className="font-medium">
                    {item.type}
                    {item.enabled === 1 && (
                      <span className="ml-2 rounded bg-green-100 px-2 py-0.5 text-xs text-green-700">
                        启用
                      </span>
                    )}
                  </div>
                  <div className="text-xs text-gray-500">{item.file_path}</div>
                  <div className="text-xs text-gray-500">
                    school_scope: {item.school_scope ?? '-'} / version: {item.version ?? '-'}
                  </div>
                </div>
                <div className="flex gap-2">
                  <button
                    className="rounded p-1.5 hover:bg-gray-200"
                    onClick={() => openEditTemplateForm(item)}
                    title="编辑模板"
                  >
                    <Edit2 size={14} />
                  </button>
                  <button
                    className="rounded p-1.5 text-red-500 hover:bg-red-100"
                    onClick={() => setDeleteTarget(item)}
                    title="删除模板"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            title="暂无模板文件"
            description="请添加模板文件"
            icon={<FileText size={32} className="text-gray-400" />}
          />
        )}
      </section>

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除模板"
        message={`确定要删除模板「${deleteTarget?.type ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={() => {
          if (deleteTarget) {
            deleteTemplateMutation.mutate(deleteTarget.id);
          }
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/** 快捷键标签页组件 */
const ShortcutTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  const [showForm, setShowForm] = useState(false);
  const [editingShortcut, setEditingShortcut] = useState<GlobalShortcut | null>(null);
  const [form, setForm] = useState<CreateGlobalShortcutInput>(INITIAL_SHORTCUT_FORM);
  const [deleteTarget, setDeleteTarget] = useState<GlobalShortcut | null>(null);

  const shortcutsQuery = useQuery({
    queryKey: ['settings', 'shortcuts'],
    queryFn: async () => unwrapResult(await commands.listGlobalShortcuts()),
  });

  const createShortcutMutation = useMutation({
    mutationFn: async (input: CreateGlobalShortcutInput) =>
      unwrapResult(await commands.createGlobalShortcut(input)),
    onSuccess: () => {
      success('快捷键已创建');
      queryClient.invalidateQueries({ queryKey: ['settings', 'shortcuts'] });
      setShowForm(false);
      setEditingShortcut(null);
      setForm(INITIAL_SHORTCUT_FORM);
    },
    onError: (err: Error) => error(err.message),
  });

  const updateShortcutMutation = useMutation({
    mutationFn: async (payload: { id: string; input: CreateGlobalShortcutInput }) => {
      const input: UpdateGlobalShortcutInput = {
        id: payload.id,
        action: payload.input.action.trim() === '' ? null : payload.input.action,
        key_combination:
          payload.input.key_combination.trim() === '' ? null : payload.input.key_combination,
        enabled: payload.input.enabled,
        description: payload.input.description,
      };
      return unwrapResult(await commands.updateGlobalShortcut(input));
    },
    onSuccess: () => {
      success('快捷键已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'shortcuts'] });
      setShowForm(false);
      setEditingShortcut(null);
      setForm(INITIAL_SHORTCUT_FORM);
    },
    onError: (err: Error) => error(err.message),
  });

  const deleteShortcutMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteGlobalShortcutInput = { id };
      return unwrapResult(await commands.deleteGlobalShortcut(input));
    },
    onSuccess: () => {
      success('快捷键已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'shortcuts'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /** 打开快捷键创建表单 */
  const openCreateShortcutForm = () => {
    setEditingShortcut(null);
    setForm(INITIAL_SHORTCUT_FORM);
    setShowForm(true);
  };

  /** 打开快捷键编辑表单 */
  const openEditShortcutForm = (item: GlobalShortcut) => {
    setEditingShortcut(item);
    setForm({
      action: item.action,
      key_combination: item.key_combination,
      enabled: item.enabled,
      description: item.description,
    });
    setShowForm(true);
  };

  /** 提交快捷键表单 */
  const submitShortcutForm = () => {
    if (editingShortcut) {
      updateShortcutMutation.mutate({ id: editingShortcut.id, input: form });
      return;
    }
    createShortcutMutation.mutate(form);
  };

  return (
    <div className="space-y-6">
      <section>
        <div className="mb-3 flex items-center justify-between">
          <h3 className="text-lg font-semibold">全局快捷键</h3>
          <button
            className="flex items-center gap-1 rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
            onClick={openCreateShortcutForm}
          >
            <Plus size={14} /> 添加快捷键
          </button>
        </div>

        {showForm && (
          <div className="mb-4 space-y-3 rounded-lg border bg-gray-50 p-4">
            <div className="grid grid-cols-2 gap-3">
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="action"
                value={form.action}
                onChange={(e) => setForm((prev) => ({ ...prev, action: e.target.value }))}
              />
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="key_combination"
                value={form.key_combination}
                onChange={(e) => setForm((prev) => ({ ...prev, key_combination: e.target.value }))}
              />
              <input
                className="col-span-2 rounded border px-3 py-2 text-sm"
                placeholder="description（可选）"
                value={form.description ?? ''}
                onChange={(e) =>
                  setForm((prev) => ({
                    ...prev,
                    description: e.target.value.trim() === '' ? null : e.target.value,
                  }))
                }
              />
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={(form.enabled ?? 0) === 1}
                onChange={(e) =>
                  setForm((prev) => ({ ...prev, enabled: e.target.checked ? 1 : 0 }))
                }
              />
              启用
            </label>
            <div className="flex gap-2">
              <button
                className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                onClick={submitShortcutForm}
              >
                {editingShortcut ? '更新' : '创建'}
              </button>
              <button
                className="rounded border px-3 py-1.5 text-sm hover:bg-gray-100"
                onClick={() => {
                  setShowForm(false);
                  setEditingShortcut(null);
                }}
              >
                取消
              </button>
            </div>
          </div>
        )}

        {shortcutsQuery.data && shortcutsQuery.data.length > 0 ? (
          <div className="space-y-2">
            {shortcutsQuery.data.map((item) => (
              <div
                key={item.id}
                className="flex items-center justify-between rounded-lg border p-3 hover:bg-gray-50"
              >
                <div>
                  <div className="flex items-center gap-2 font-medium">
                    {item.enabled === 1 ? (
                      <CheckCircle2 size={16} className="text-green-500" />
                    ) : (
                      <XCircle size={16} className="text-gray-400" />
                    )}
                    {item.action}
                  </div>
                  <div className="mt-1">
                    <kbd className="rounded border bg-gray-100 px-2 py-0.5 font-mono text-xs">
                      {item.key_combination}
                    </kbd>
                  </div>
                  <div className="text-xs text-gray-500">{item.description ?? '无描述'}</div>
                </div>
                <div className="flex gap-2">
                  <button
                    className="rounded p-1.5 hover:bg-gray-200"
                    onClick={() => openEditShortcutForm(item)}
                    title="编辑快捷键"
                  >
                    <Edit2 size={14} />
                  </button>
                  <button
                    className="rounded p-1.5 text-red-500 hover:bg-red-100"
                    onClick={() => setDeleteTarget(item)}
                    title="删除快捷键"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            title="暂无快捷键"
            description="请添加全局快捷键"
            icon={<Keyboard size={32} className="text-gray-400" />}
          />
        )}
      </section>

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除快捷键"
        message={`确定要删除「${deleteTarget?.action ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={() => {
          if (deleteTarget) {
            deleteShortcutMutation.mutate(deleteTarget.id);
          }
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/** Skills 与 MCP 标签页组件 */
const SkillsMcpTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error, info } = useToast();

  const [subTab, setSubTab] = useState<'skills' | 'uv' | 'mcp'>('skills');

  const [showSkillForm, setShowSkillForm] = useState(false);
  const [editingSkill, setEditingSkill] = useState<SkillRecord | null>(null);
  const [skillForm, setSkillForm] = useState<CreateSkillInput>(INITIAL_SKILL_FORM);
  const [skillStatus, setSkillStatus] = useState<string | null>('active');

  const [showMcpForm, setShowMcpForm] = useState(false);
  const [editingMcp, setEditingMcp] = useState<McpServerRecord | null>(null);
  const [mcpForm, setMcpForm] = useState<CreateMcpServerInput>(INITIAL_MCP_FORM);
  const [mcpEnabled, setMcpEnabled] = useState<number | null>(1);

  const [deleteTarget, setDeleteTarget] = useState<{
    type: 'skill' | 'mcp';
    id: string;
    name: string;
  } | null>(null);

  const [skillEnvName, setSkillEnvName] = useState('');
  const [skillEnvPythonVersion, setSkillEnvPythonVersion] = useState<string>('');

  const skillsQuery = useQuery({
    queryKey: ['settings', 'skills'],
    queryFn: async () => unwrapResult(await commands.listSkills()),
  });

  const uvHealthQuery = useQuery({
    queryKey: ['settings', 'uv-health'],
    queryFn: async (): Promise<UvHealthResult> => unwrapResult(await commands.checkUvHealth()),
  });

  const mcpQuery = useQuery({
    queryKey: ['settings', 'mcp-servers'],
    queryFn: async () => unwrapResult(await commands.listMcpServers()),
  });

  const createSkillMutation = useMutation({
    mutationFn: async (input: CreateSkillInput) => unwrapResult(await commands.createSkill(input)),
    onSuccess: () => {
      success('技能已创建');
      queryClient.invalidateQueries({ queryKey: ['settings', 'skills'] });
      setShowSkillForm(false);
      setEditingSkill(null);
      setSkillForm(INITIAL_SKILL_FORM);
      setSkillStatus('active');
    },
    onError: (err: Error) => error(err.message),
  });

  const updateSkillMutation = useMutation({
    mutationFn: async (payload: { id: string; input: UpdateSkillInput }) =>
      unwrapResult(await commands.updateSkill(payload.id, payload.input)),
    onSuccess: () => {
      success('技能已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'skills'] });
      setShowSkillForm(false);
      setEditingSkill(null);
      setSkillForm(INITIAL_SKILL_FORM);
      setSkillStatus('active');
    },
    onError: (err: Error) => error(err.message),
  });

  const deleteSkillMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteSkillInput = { id };
      return unwrapResult(await commands.deleteSkill(input));
    },
    onSuccess: () => {
      success('技能已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'skills'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const checkSkillHealthMutation = useMutation({
    mutationFn: async (id: string) => unwrapResult(await commands.checkSkillHealth(id)),
    onSuccess: (data) => {
      if (data.health_status === 'healthy') {
        success(`技能「${data.name}」健康检查通过`);
      } else {
        error(`技能「${data.name}」异常：${data.message}`);
      }
      queryClient.invalidateQueries({ queryKey: ['settings', 'skills'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const installUvMutation = useMutation({
    mutationFn: async () => unwrapResult(await commands.installUv()),
    onSuccess: (data) => {
      if (data.success) {
        success(`uv 安装成功，版本：${data.version ?? '未知'}`);
      } else {
        error(`uv 安装失败：${data.output}`);
      }
      queryClient.invalidateQueries({ queryKey: ['settings', 'uv-health'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const createSkillEnvMutation = useMutation({
    mutationFn: async (input: CreateSkillEnvInput) =>
      unwrapResult(await commands.createSkillEnv(input)),
    onSuccess: (message) => {
      info(message);
    },
    onError: (err: Error) => error(err.message),
  });

  const createMcpMutation = useMutation({
    mutationFn: async (input: CreateMcpServerInput) =>
      unwrapResult(await commands.createMcpServer(input)),
    onSuccess: () => {
      success('MCP 服务器已创建');
      queryClient.invalidateQueries({ queryKey: ['settings', 'mcp-servers'] });
      setShowMcpForm(false);
      setEditingMcp(null);
      setMcpForm(INITIAL_MCP_FORM);
      setMcpEnabled(1);
    },
    onError: (err: Error) => error(err.message),
  });

  const updateMcpMutation = useMutation({
    mutationFn: async (payload: { id: string; input: UpdateMcpServerInput }) =>
      unwrapResult(await commands.updateMcpServer(payload.id, payload.input)),
    onSuccess: () => {
      success('MCP 服务器已更新');
      queryClient.invalidateQueries({ queryKey: ['settings', 'mcp-servers'] });
      setShowMcpForm(false);
      setEditingMcp(null);
      setMcpForm(INITIAL_MCP_FORM);
      setMcpEnabled(1);
    },
    onError: (err: Error) => error(err.message),
  });

  const deleteMcpMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteMcpServerInput = { id };
      return unwrapResult(await commands.deleteMcpServer(input));
    },
    onSuccess: () => {
      success('MCP 服务器已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'mcp-servers'] });
    },
    onError: (err: Error) => error(err.message),
  });

  const checkMcpHealthMutation = useMutation({
    mutationFn: async (id: string) => unwrapResult(await commands.checkMcpHealth(id)),
    onSuccess: (data) => {
      if (data.health_status === 'healthy') {
        success(`MCP「${data.name}」健康检查通过`);
      } else {
        error(`MCP「${data.name}」异常：${data.message}`);
      }
      queryClient.invalidateQueries({ queryKey: ['settings', 'mcp-servers'] });
    },
    onError: (err: Error) => error(err.message),
  });

  /** 打开技能创建表单 */
  const openCreateSkillForm = () => {
    setEditingSkill(null);
    setSkillForm(INITIAL_SKILL_FORM);
    setSkillStatus('active');
    setShowSkillForm(true);
  };

  /** 打开技能编辑表单 */
  const openEditSkillForm = (item: SkillRecord) => {
    setEditingSkill(item);
    setSkillForm({
      name: item.name,
      version: item.version,
      source: item.source,
      permission_scope: item.permission_scope,
      display_name: item.display_name,
      description: item.description,
      skill_type: item.skill_type,
      env_path: item.env_path,
      config_json: item.config_json,
      license: item.license,
      compatibility: item.compatibility,
      metadata_json: item.metadata_json,
      allowed_tools: item.allowed_tools,
      body_content: item.body_content,
      entry_script: item.entry_script,
    });
    setSkillStatus(item.status);
    setShowSkillForm(true);
  };

  /** 提交技能表单 */
  const submitSkillForm = () => {
    if (editingSkill) {
      const input: UpdateSkillInput = {
        display_name: skillForm.display_name,
        description: skillForm.description,
        permission_scope: skillForm.permission_scope,
        config_json: skillForm.config_json,
        status: skillStatus,
        license: skillForm.license,
        compatibility: skillForm.compatibility,
        metadata_json: skillForm.metadata_json,
        allowed_tools: skillForm.allowed_tools,
        body_content: skillForm.body_content,
        entry_script: skillForm.entry_script,
      };
      updateSkillMutation.mutate({ id: editingSkill.id, input });
      return;
    }
    createSkillMutation.mutate(skillForm);
  };

  /** 打开 MCP 创建表单 */
  const openCreateMcpForm = () => {
    setEditingMcp(null);
    setMcpForm(INITIAL_MCP_FORM);
    setMcpEnabled(1);
    setShowMcpForm(true);
  };

  /** 打开 MCP 编辑表单 */
  const openEditMcpForm = (item: McpServerRecord) => {
    setEditingMcp(item);
    setMcpForm({
      name: item.name,
      transport: item.transport,
      command: item.command,
      args_json: item.args_json,
      env_json: item.env_json,
      permission_scope: item.permission_scope,
      display_name: item.display_name,
      description: item.description,
    });
    setMcpEnabled(item.enabled);
    setShowMcpForm(true);
  };

  /** 提交 MCP 表单 */
  const submitMcpForm = () => {
    if (editingMcp) {
      const input: UpdateMcpServerInput = {
        display_name: mcpForm.display_name,
        description: mcpForm.description,
        command: mcpForm.command,
        args_json: mcpForm.args_json,
        env_json: mcpForm.env_json,
        permission_scope: mcpForm.permission_scope,
        enabled: mcpEnabled,
      };
      updateMcpMutation.mutate({ id: editingMcp.id, input });
      return;
    }
    createMcpMutation.mutate(mcpForm);
  };

  /** 提交技能环境创建 */
  const submitCreateSkillEnv = () => {
    if (skillEnvName.trim() === '') {
      error('请先输入技能名称');
      return;
    }
    const input: CreateSkillEnvInput = {
      skill_name: skillEnvName,
      python_version: skillEnvPythonVersion.trim() === '' ? null : skillEnvPythonVersion,
    };
    createSkillEnvMutation.mutate(input);
  };

  /** 确认删除技能或 MCP */
  const confirmDelete = () => {
    if (!deleteTarget) {
      return;
    }
    if (deleteTarget.type === 'skill') {
      deleteSkillMutation.mutate(deleteTarget.id);
    } else {
      deleteMcpMutation.mutate(deleteTarget.id);
    }
    setDeleteTarget(null);
  };

  return (
    <div className="space-y-4">
      <div className="flex gap-1 border-b">
        {(['skills', 'uv', 'mcp'] as const).map((tab) => (
          <button
            key={tab}
            className={`border-b-2 px-4 py-2 text-sm transition-colors ${
              subTab === tab
                ? 'border-blue-600 font-medium text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700'
            }`}
            onClick={() => setSubTab(tab)}
          >
            {tab === 'skills' ? '技能管理' : tab === 'uv' ? 'Python环境(uv)' : 'MCP服务器'}
          </button>
        ))}
      </div>

      {subTab === 'skills' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">技能列表</h3>
            <button
              className="flex items-center gap-1 rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
              onClick={openCreateSkillForm}
            >
              <Plus size={14} /> 添加技能
            </button>
          </div>

          {showSkillForm && (
            <div className="space-y-3 rounded-lg border bg-gray-50 p-4">
              <div className="grid grid-cols-2 gap-3">
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="name"
                  value={skillForm.name}
                  disabled={editingSkill !== null}
                  onChange={(e) => setSkillForm((prev) => ({ ...prev, name: e.target.value }))}
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="display_name（可选）"
                  value={skillForm.display_name ?? ''}
                  onChange={(e) =>
                    setSkillForm((prev) => ({
                      ...prev,
                      display_name: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="version（可选）"
                  value={skillForm.version ?? ''}
                  onChange={(e) =>
                    setSkillForm((prev) => ({
                      ...prev,
                      version: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="source（可选）"
                  value={skillForm.source ?? ''}
                  onChange={(e) =>
                    setSkillForm((prev) => ({
                      ...prev,
                      source: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="permission_scope（可选）"
                  value={skillForm.permission_scope ?? ''}
                  onChange={(e) =>
                    setSkillForm((prev) => ({
                      ...prev,
                      permission_scope: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="skill_type"
                  value={skillForm.skill_type}
                  onChange={(e) =>
                    setSkillForm((prev) => ({ ...prev, skill_type: e.target.value }))
                  }
                />
                <input
                  className="col-span-2 rounded border px-3 py-2 text-sm"
                  placeholder="description（可选）"
                  value={skillForm.description ?? ''}
                  onChange={(e) =>
                    setSkillForm((prev) => ({
                      ...prev,
                      description: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="col-span-2 rounded border px-3 py-2 text-sm"
                  placeholder="config_json（可选）"
                  value={skillForm.config_json ?? ''}
                  onChange={(e) =>
                    setSkillForm((prev) => ({
                      ...prev,
                      config_json: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
              </div>
              {editingSkill && (
                <select
                  className="rounded border px-3 py-2 text-sm"
                  value={skillStatus ?? ''}
                  onChange={(e) =>
                    setSkillStatus(e.target.value.trim() === '' ? null : e.target.value)
                  }
                >
                  <option value="active">active</option>
                  <option value="inactive">inactive</option>
                </select>
              )}
              <div className="flex gap-2">
                <button
                  className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                  onClick={submitSkillForm}
                >
                  {editingSkill ? '更新' : '创建'}
                </button>
                <button
                  className="rounded border px-3 py-1.5 text-sm hover:bg-gray-100"
                  onClick={() => {
                    setShowSkillForm(false);
                    setEditingSkill(null);
                  }}
                >
                  取消
                </button>
              </div>
            </div>
          )}

          {skillsQuery.data && skillsQuery.data.length > 0 ? (
            <div className="space-y-2">
              {skillsQuery.data.map((item) => (
                <div
                  key={item.id}
                  className="flex items-center justify-between rounded-lg border p-3 hover:bg-gray-50"
                >
                  <div>
                    <div className="flex items-center gap-2 font-medium">
                      {item.status === 'active' ? (
                        <CheckCircle2 size={16} className="text-green-500" />
                      ) : (
                        <XCircle size={16} className="text-gray-400" />
                      )}
                      {item.display_name ?? item.name}
                      <span className="rounded bg-gray-100 px-2 py-0.5 text-xs">
                        {item.skill_type}
                      </span>
                      <span
                        className={`rounded px-2 py-0.5 text-xs ${
                          item.health_status === 'healthy'
                            ? 'bg-green-100 text-green-700'
                            : 'bg-red-100 text-red-700'
                        }`}
                      >
                        {item.health_status}
                      </span>
                    </div>
                    <div className="text-xs text-gray-500">name: {item.name}</div>
                    <div className="text-xs text-gray-500">
                      description: {item.description ?? '-'}
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button
                      className="rounded p-1.5 text-blue-600 hover:bg-blue-100"
                      onClick={() => checkSkillHealthMutation.mutate(item.id)}
                      title="健康检查"
                    >
                      <Activity size={14} />
                    </button>
                    <button
                      className="rounded p-1.5 hover:bg-gray-200"
                      onClick={() => openEditSkillForm(item)}
                      title="编辑技能"
                    >
                      <Edit2 size={14} />
                    </button>
                    <button
                      className="rounded p-1.5 text-red-500 hover:bg-red-100"
                      onClick={() =>
                        setDeleteTarget({
                          type: 'skill',
                          id: item.id,
                          name: item.display_name ?? item.name,
                        })
                      }
                      title="删除技能"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState
              title="暂无技能"
              description="请添加技能配置"
              icon={<Puzzle size={32} className="text-gray-400" />}
            />
          )}
        </div>
      )}

      {subTab === 'uv' && (
        <div className="space-y-4">
          <h3 className="text-lg font-semibold">Python 环境（uv）</h3>
          <div className="space-y-3 rounded-lg border p-4">
            <div className="flex items-center gap-2 text-sm">
              {uvHealthQuery.data?.available ? (
                <CheckCircle2 size={16} className="text-green-500" />
              ) : (
                <XCircle size={16} className="text-red-500" />
              )}
              {uvHealthQuery.data?.available ? 'uv 已安装' : 'uv 未安装'}
            </div>
            <div className="text-xs text-gray-500">版本：{uvHealthQuery.data?.version ?? '-'}</div>
            <div className="text-xs text-gray-500">路径：{uvHealthQuery.data?.path ?? '-'}</div>
            <div className="text-xs text-gray-500">信息：{uvHealthQuery.data?.message ?? '-'}</div>
            <div className="flex gap-2">
              <button
                className="flex items-center gap-1 rounded border px-3 py-1.5 text-sm hover:bg-gray-50"
                onClick={() =>
                  queryClient.invalidateQueries({ queryKey: ['settings', 'uv-health'] })
                }
              >
                <RefreshCw size={14} /> 重新检测
              </button>
              <button
                className="flex items-center gap-1 rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                onClick={() => installUvMutation.mutate()}
              >
                <Download size={14} /> 安装 uv
              </button>
            </div>
          </div>

          <div className="space-y-3 rounded-lg border p-4">
            <h4 className="font-medium">创建技能环境</h4>
            <div className="grid grid-cols-2 gap-3">
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="skill_name"
                value={skillEnvName}
                onChange={(e) => setSkillEnvName(e.target.value)}
              />
              <input
                className="rounded border px-3 py-2 text-sm"
                placeholder="python_version（可选）"
                value={skillEnvPythonVersion}
                onChange={(e) => setSkillEnvPythonVersion(e.target.value)}
              />
            </div>
            <button
              className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
              onClick={submitCreateSkillEnv}
            >
              创建环境
            </button>
          </div>
        </div>
      )}

      {subTab === 'mcp' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">MCP 服务器</h3>
            <button
              className="flex items-center gap-1 rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
              onClick={openCreateMcpForm}
            >
              <Plus size={14} /> 添加 MCP
            </button>
          </div>

          {showMcpForm && (
            <div className="space-y-3 rounded-lg border bg-gray-50 p-4">
              <div className="grid grid-cols-2 gap-3">
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="name"
                  value={mcpForm.name}
                  disabled={editingMcp !== null}
                  onChange={(e) => setMcpForm((prev) => ({ ...prev, name: e.target.value }))}
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="display_name（可选）"
                  value={mcpForm.display_name ?? ''}
                  onChange={(e) =>
                    setMcpForm((prev) => ({
                      ...prev,
                      display_name: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="transport"
                  value={mcpForm.transport}
                  disabled={editingMcp !== null}
                  onChange={(e) => setMcpForm((prev) => ({ ...prev, transport: e.target.value }))}
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="command（可选）"
                  value={mcpForm.command ?? ''}
                  onChange={(e) =>
                    setMcpForm((prev) => ({
                      ...prev,
                      command: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="args_json（可选）"
                  value={mcpForm.args_json ?? ''}
                  onChange={(e) =>
                    setMcpForm((prev) => ({
                      ...prev,
                      args_json: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="env_json（可选）"
                  value={mcpForm.env_json ?? ''}
                  onChange={(e) =>
                    setMcpForm((prev) => ({
                      ...prev,
                      env_json: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="permission_scope（可选）"
                  value={mcpForm.permission_scope ?? ''}
                  onChange={(e) =>
                    setMcpForm((prev) => ({
                      ...prev,
                      permission_scope: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
                <input
                  className="rounded border px-3 py-2 text-sm"
                  placeholder="description（可选）"
                  value={mcpForm.description ?? ''}
                  onChange={(e) =>
                    setMcpForm((prev) => ({
                      ...prev,
                      description: e.target.value.trim() === '' ? null : e.target.value,
                    }))
                  }
                />
              </div>
              {editingMcp && (
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={(mcpEnabled ?? 0) === 1}
                    onChange={(e) => setMcpEnabled(e.target.checked ? 1 : 0)}
                  />
                  启用
                </label>
              )}
              <div className="flex gap-2">
                <button
                  className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                  onClick={submitMcpForm}
                >
                  {editingMcp ? '更新' : '创建'}
                </button>
                <button
                  className="rounded border px-3 py-1.5 text-sm hover:bg-gray-100"
                  onClick={() => {
                    setShowMcpForm(false);
                    setEditingMcp(null);
                  }}
                >
                  取消
                </button>
              </div>
            </div>
          )}

          {mcpQuery.data && mcpQuery.data.length > 0 ? (
            <div className="space-y-2">
              {mcpQuery.data.map((item) => (
                <div
                  key={item.id}
                  className="flex items-center justify-between rounded-lg border p-3 hover:bg-gray-50"
                >
                  <div>
                    <div className="flex items-center gap-2 font-medium">
                      {item.enabled === 1 ? (
                        <CheckCircle2 size={16} className="text-green-500" />
                      ) : (
                        <XCircle size={16} className="text-gray-400" />
                      )}
                      {item.display_name ?? item.name}
                      <span className="rounded bg-gray-100 px-2 py-0.5 text-xs">
                        {item.transport}
                      </span>
                      <span
                        className={`rounded px-2 py-0.5 text-xs ${
                          item.health_status === 'healthy'
                            ? 'bg-green-100 text-green-700'
                            : 'bg-red-100 text-red-700'
                        }`}
                      >
                        {item.health_status}
                      </span>
                    </div>
                    <div className="text-xs text-gray-500">command: {item.command ?? '-'}</div>
                    <div className="text-xs text-gray-500">args_json: {item.args_json ?? '-'}</div>
                  </div>
                  <div className="flex gap-2">
                    <button
                      className="rounded p-1.5 text-blue-600 hover:bg-blue-100"
                      onClick={() => checkMcpHealthMutation.mutate(item.id)}
                      title="健康检查"
                    >
                      <Activity size={14} />
                    </button>
                    <button
                      className="rounded p-1.5 hover:bg-gray-200"
                      onClick={() => openEditMcpForm(item)}
                      title="编辑 MCP"
                    >
                      <Edit2 size={14} />
                    </button>
                    <button
                      className="rounded p-1.5 text-red-500 hover:bg-red-100"
                      onClick={() =>
                        setDeleteTarget({
                          type: 'mcp',
                          id: item.id,
                          name: item.display_name ?? item.name,
                        })
                      }
                      title="删除 MCP"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState
              title="暂无 MCP 服务器"
              description="请添加 MCP 服务配置"
              icon={<Puzzle size={32} className="text-gray-400" />}
            />
          )}
        </div>
      )}

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除"
        message={`确定要删除「${deleteTarget?.name ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};

/** 系统设置页面主组件 */
export const SettingsPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabKey>('ai');

  /** 渲染当前激活标签页内容 */
  const renderContent = () => {
    if (activeTab === 'ai') {
      return <AiConfigTab />;
    }
    if (activeTab === 'security') {
      return <SecurityTab />;
    }
    if (activeTab === 'template') {
      return <TemplateTab />;
    }
    if (activeTab === 'shortcut') {
      return <ShortcutTab />;
    }
    return <SkillsMcpTab />;
  };

  return (
    <div className="flex h-full flex-col">
      <div className="mb-4 flex items-center gap-2">
        <Settings size={20} />
        <h2 className="text-xl font-bold">系统设置</h2>
      </div>

      <div className="mb-6 flex gap-1 border-b">
        {[
          { key: 'ai' as const, label: 'AI配置', icon: <Cpu size={16} /> },
          { key: 'security' as const, label: '安全隐私', icon: <Shield size={16} /> },
          { key: 'template' as const, label: '模板导出', icon: <FileText size={16} /> },
          { key: 'shortcut' as const, label: '快捷键', icon: <Keyboard size={16} /> },
          { key: 'skills' as const, label: 'Skills与MCP', icon: <Puzzle size={16} /> },
        ].map((tab) => (
          <button
            key={tab.key}
            className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-sm transition-colors ${
              activeTab === tab.key
                ? 'border-blue-600 font-medium text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700'
            }`}
            onClick={() => setActiveTab(tab.key)}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">{renderContent()}</div>
    </div>
  );
};
