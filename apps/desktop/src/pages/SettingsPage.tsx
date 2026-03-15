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
  type AppError,
  type DeleteAiConfigInput,
  type DeleteAiParamPresetInput,
  type DeleteSkillInput,
  type InstallFromGitInput,
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
  RefreshCw,
  Loader2,
  Check,
  Bot,
  Sparkles,
  Zap,
  Github,
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
    // 显示已保存的模型作为占位符（只读显示）
    if (item.default_model) {
      setModels([
        {
          id: item.default_model,
          name: item.default_model,
          is_vision: false,
        },
      ]);
    } else {
      setModels([]);
    }
    setConfigForm({
      provider_name: item.provider_name,
      display_name: item.display_name,
      base_url: item.base_url,
      api_key: '', // API Key 需要重新输入才能刷新模型列表
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

            {/* 编辑配置时的提示 */}
            {editingConfig && models.length === 1 && models[0].id === editingConfig.default_model && (
              <div className="rounded-lg border border-blue-200 bg-blue-50 p-3 text-sm text-blue-800">
                <p className="font-medium mb-1">当前已选择模型</p>
                <p>上方显示的是当前已保存的模型。如需更换模型，请输入 API Key 并点击"获取模型列表"刷新。</p>
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

/** SecurityTab 组件 */
const SecurityTab: React.FC = () => {
  return (
    <div className="space-y-6">
      <section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="mb-4 text-lg font-semibold text-gray-900">日志与诊断</h3>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">日志文件位置</label>
            <div className="p-3 bg-gray-50 rounded-lg border">
              <code className="text-sm text-gray-600 break-all">
                Windows: %APPDATA%/com.pureworker/logs/
              </code>
            </div>
            <p className="mt-2 text-xs text-gray-500">
              Windows 系统日志路径示例：C:\Users\用户名\AppData\Roaming\com.pureworker\logs\
            </p>
          </div>
        </div>
      </section>
    </div>
  );
};

/** TemplateTab 组件 */
const TemplateTab: React.FC = () => {
  return (
    <div className="space-y-4">
      <div className="text-gray-500">模板导出功能开发中。</div>
    </div>
  );
};

/** ShortcutTab 组件 */
const ShortcutTab: React.FC = () => {
  return (
    <div className="space-y-4">
      <div className="text-gray-500">快捷键设置功能开发中。</div>
    </div>
  );
};

/** Skills 与 MCP 标签页组件 */
const SkillsMcpTab: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  const [showGitImport, setShowGitImport] = useState(false);
  const [gitUrl, setGitUrl] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<{
    type: 'skill' | 'mcp';
    id: string;
    name: string;
  } | null>(null);

  const skillsQuery = useQuery({
    queryKey: ['settings', 'skills'],
    queryFn: async () => {
      const result = await commands.listSkills();
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error('获取技能列表失败');
    },
  });

  const installFromGitMutation = useMutation({
    mutationFn: async (gitUrl: string) => {
      const input: InstallFromGitInput = {
        git_url: gitUrl,
        workspace_path: '', // 使用默认工作区
      };
      const result = await commands.installStoreSkillFromGit(input);
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error(getErrorMessage(result.error));
    },
    onSuccess: () => {
      success('技能已从 GitHub 导入');
      queryClient.invalidateQueries({ queryKey: ['settings', 'skills'] });
      setShowGitImport(false);
      setGitUrl('');
    },
    onError: (err: Error) => error(err.message),
  });

  const checkSkillHealthMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.checkSkillHealth(id);
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error(getErrorMessage(result.error));
    },
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

  const deleteSkillMutation = useMutation({
    mutationFn: async (id: string) => {
      const input: DeleteSkillInput = { id };
      const result = await commands.deleteSkill(input);
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error(getErrorMessage(result.error));
    },
    onSuccess: () => {
      success('技能已删除');
      queryClient.invalidateQueries({ queryKey: ['settings', 'skills'] });
      setDeleteTarget(null);
    },
    onError: (err: Error) => error(err.message),
  });

  const handleInstallFromGit = () => {
    if (!gitUrl.trim()) {
      error('请输入 Git 仓库 URL');
      return;
    }
    installFromGitMutation.mutate(gitUrl.trim());
  };

  return (
    <div className="space-y-6">
      <section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="text-lg font-semibold">技能列表</h3>
          <div className="flex gap-2">
            <button
              className="flex items-center gap-1 rounded border px-3 py-1.5 text-sm hover:bg-gray-50"
              onClick={() => setShowGitImport(true)}
            >
              <Github size={14} /> 从 GitHub 导入
            </button>
          </div>
        </div>

        {showGitImport && (
          <div className="mb-4 space-y-3 rounded-lg border bg-gray-50 p-4">
            <h4 className="font-medium">从 Git 仓库导入技能</h4>
            <input
              className="w-full rounded border px-3 py-2 text-sm"
              placeholder="Git 仓库 URL（仅支持 github.com / gitee.com）"
              value={gitUrl}
              onChange={(e) => setGitUrl(e.target.value)}
            />
            <div className="flex gap-2">
              <button
                className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                onClick={handleInstallFromGit}
                disabled={!gitUrl.trim() || installFromGitMutation.isPending}
              >
                {installFromGitMutation.isPending ? '导入中...' : '导入'}
              </button>
              <button
                className="rounded border px-3 py-1.5 text-sm hover:bg-gray-100"
                onClick={() => {
                  setShowGitImport(false);
                  setGitUrl('');
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
            description="点击上方按钮从 GitHub 导入技能"
            icon={<Puzzle size={32} className="text-gray-400" />}
          />
        )}
      </section>

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除"
        message={`确定要删除技能「${deleteTarget?.name ?? ''}」吗？`}
        confirmText="删除"
        isDestructive
        onConfirm={() => deleteTarget && deleteSkillMutation.mutate(deleteTarget.id)}
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
