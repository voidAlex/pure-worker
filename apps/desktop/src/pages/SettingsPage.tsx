/**
 * AI 配置管理页面组件
 * 展示 AI 服务商配置列表，支持创建、编辑、删除配置
 */

import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands, AiConfigSafe, CreateAiConfigInput, UpdateAiConfigInput } from '@/bindings';
import { useToast } from '@/hooks/useToast';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import { EmptyState } from '@/components/shared/EmptyState';
import { Plus, Edit2, Trash2, Settings, Eye, EyeOff } from 'lucide-react';

export const SettingsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingConfig, setEditingConfig] = useState<AiConfigSafe | null>(null);
  const [deletingConfig, setDeletingConfig] = useState<AiConfigSafe | null>(null);
  const [showApiKey, setShowApiKey] = useState(false);

  const [formData, setFormData] = useState<{
    provider_name: string;
    display_name: string;
    base_url: string;
    api_key: string;
    default_model: string;
    is_active: boolean;
    config_json: string;
  }>({
    provider_name: '',
    display_name: '',
    base_url: '',
    api_key: '',
    default_model: '',
    is_active: true,
    config_json: '',
  });

  const { data: configs, isLoading } = useQuery({
    queryKey: ['aiConfigs'],
    queryFn: async () => {
      const result = await commands.listAiConfigs();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const createMutation = useMutation({
    mutationFn: async (input: CreateAiConfigInput) => {
      const result = await commands.createAiConfig(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['aiConfigs'] });
      success('配置创建成功');
      setIsModalOpen(false);
      resetForm();
    },
    onError: (err) => error(`创建失败: ${err.message}`),
  });

  const updateMutation = useMutation({
    mutationFn: async (input: UpdateAiConfigInput) => {
      const result = await commands.updateAiConfig(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['aiConfigs'] });
      success('配置更新成功');
      setIsModalOpen(false);
      resetForm();
    },
    onError: (err) => error(`更新失败: ${err.message}`),
  });

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.deleteAiConfig({ id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['aiConfigs'] });
      success('配置删除成功');
      setDeletingConfig(null);
    },
    onError: (err) => error(`删除失败: ${err.message}`),
  });

  /**
   * 重置表单状态
   */
  const resetForm = () => {
    setFormData({
      provider_name: '',
      display_name: '',
      base_url: '',
      api_key: '',
      default_model: '',
      is_active: true,
      config_json: '',
    });
    setEditingConfig(null);
    setShowApiKey(false);
  };

  /**
   * 打开创建/编辑弹窗
   * @param config 可选，如果有值则为编辑模式，否则为创建模式
   */
  const handleOpenModal = (config?: AiConfigSafe) => {
    if (config) {
      setEditingConfig(config);
      setFormData({
        provider_name: config.provider_name,
        display_name: config.display_name,
        base_url: config.base_url,
        api_key: '', // Do not populate API key on edit
        default_model: config.default_model,
        is_active: config.is_active === 1,
        config_json: config.config_json || '',
      });
    } else {
      resetForm();
    }
    setIsModalOpen(true);
  };

  /**
   * 提交表单
   * @param e 表单提交事件
   */
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (editingConfig) {
      updateMutation.mutate({
        id: editingConfig.id,
        display_name: formData.display_name !== editingConfig.display_name ? formData.display_name : null,
        base_url: formData.base_url !== editingConfig.base_url ? formData.base_url : null,
        api_key: formData.api_key ? formData.api_key : null,
        default_model: formData.default_model !== editingConfig.default_model ? formData.default_model : null,
        is_active: formData.is_active !== (editingConfig.is_active === 1) ? formData.is_active : null,
        config_json: formData.config_json !== (editingConfig.config_json || '') ? (formData.config_json || null) : null,
      });
    } else {
      createMutation.mutate({
        provider_name: formData.provider_name,
        display_name: formData.display_name,
        base_url: formData.base_url,
        api_key: formData.api_key,
        default_model: formData.default_model,
        is_active: formData.is_active,
        config_json: formData.config_json || null,
      });
    }
  };

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">AI 配置</h1>
          <p className="text-sm text-gray-500 mt-1">管理 AI 服务商及模型配置</p>
        </div>
        <button
          onClick={() => handleOpenModal()}
          className="flex items-center gap-2 px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium"
        >
          <Plus className="w-4 h-4" />
          添加配置
        </button>
      </header>

      {isLoading ? (
        <div className="p-8 text-center text-gray-500">加载中...</div>
      ) : configs && configs.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {configs.map((config) => (
            <div key={config.id} className="bg-white rounded-lg border border-gray-200 p-4 shadow-sm flex flex-col">
              <div className="flex justify-between items-start mb-3">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900">{config.display_name}</h3>
                  <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-800 mt-1">
                    {config.provider_name}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <span className={`w-2.5 h-2.5 rounded-full ${config.is_active === 1 ? 'bg-green-500' : 'bg-gray-300'}`} title={config.is_active === 1 ? '已启用' : '已禁用'} />
                </div>
              </div>
              
              <div className="space-y-2 text-sm text-gray-600 flex-1">
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">API 地址:</span>
                  <span className="truncate ml-2 max-w-[180px]" title={config.base_url}>{config.base_url}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">默认模型:</span>
                  <span className="truncate ml-2 max-w-[180px]" title={config.default_model}>{config.default_model}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-500">API 密钥:</span>
                  {config.has_api_key ? (
                    <span className="text-green-600 font-medium">已配置</span>
                  ) : (
                    <span className="text-orange-500 font-medium">未配置</span>
                  )}
                </div>
              </div>

              <div className="mt-4 pt-4 border-t border-gray-100 flex justify-end gap-2">
                <button
                  onClick={() => handleOpenModal(config)}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-600 hover:text-brand-600 hover:bg-brand-50 rounded-md transition-colors"
                >
                  <Edit2 className="w-3.5 h-3.5" />
                  编辑
                </button>
                <button
                  onClick={() => setDeletingConfig(config)}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-600 hover:text-red-600 hover:bg-red-50 rounded-md transition-colors"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  删除
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <EmptyState
          icon={<Settings className="w-8 h-8" />}
          title="暂无配置"
          description="您还没有添加任何 AI 服务商配置，点击右上角按钮添加配置。"
          action={
            <button
              onClick={() => handleOpenModal()}
              className="flex items-center gap-2 px-4 py-2 bg-brand-50 text-brand-700 rounded-lg hover:bg-brand-100 transition-colors font-medium"
            >
              <Plus className="w-4 h-4" />
              添加配置
            </button>
          }
        />
      )}

      {/* Form Modal */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
          <div className="bg-white rounded-xl shadow-2xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-200">
            <div className="px-6 py-4 border-b border-gray-100 flex justify-between items-center">
              <h3 className="text-lg font-semibold text-gray-900">
                {editingConfig ? '编辑配置' : '添加配置'}
              </h3>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                &times;
              </button>
            </div>
            <form onSubmit={handleSubmit} className="p-6 space-y-4 max-h-[70vh] overflow-y-auto">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">服务商标识</label>
                <input
                  type="text"
                  required
                  disabled={!!editingConfig}
                  value={formData.provider_name}
                  onChange={(e) => setFormData({ ...formData, provider_name: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow disabled:bg-gray-100 disabled:text-gray-500"
                  placeholder="例如：deepseek"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">显示名称</label>
                <input
                  type="text"
                  required
                  value={formData.display_name}
                  onChange={(e) => setFormData({ ...formData, display_name: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                  placeholder="例如：DeepSeek V3"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">API 地址</label>
                <input
                  type="text"
                  required
                  value={formData.base_url}
                  onChange={(e) => setFormData({ ...formData, base_url: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                  placeholder="例如：https://api.deepseek.com/v1"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">API 密钥</label>
                <div className="relative">
                  <input
                    type={showApiKey ? "text" : "password"}
                    required={!editingConfig}
                    value={formData.api_key}
                    onChange={(e) => setFormData({ ...formData, api_key: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow pr-10"
                    placeholder={editingConfig ? "留空表示不修改" : "输入 API 密钥"}
                  />
                  <button
                    type="button"
                    onClick={() => setShowApiKey(!showApiKey)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-gray-400 hover:text-gray-600"
                  >
                    {showApiKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                  </button>
                </div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">默认模型</label>
                <input
                  type="text"
                  required
                  value={formData.default_model}
                  onChange={(e) => setFormData({ ...formData, default_model: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                  placeholder="例如：deepseek-chat"
                />
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="is_active"
                  checked={formData.is_active}
                  onChange={(e) => setFormData({ ...formData, is_active: e.target.checked })}
                  className="w-4 h-4 text-brand-600 border-gray-300 rounded focus:ring-brand-500"
                />
                <label htmlFor="is_active" className="text-sm font-medium text-gray-700">启用</label>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">扩展配置 (JSON)</label>
                <textarea
                  value={formData.config_json}
                  onChange={(e) => setFormData({ ...formData, config_json: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow font-mono text-sm"
                  placeholder="{}"
                  rows={3}
                />
              </div>
              <div className="pt-4 flex justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setIsModalOpen(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={createMutation.isPending || updateMutation.isPending}
                  className="px-4 py-2 text-sm font-medium text-white bg-brand-600 rounded-lg hover:bg-brand-700 disabled:opacity-50"
                >
                  {createMutation.isPending || updateMutation.isPending ? '保存中...' : '保存'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      <ConfirmDialog
        isOpen={!!deletingConfig}
        title="删除配置"
        message={`确定要删除「${deletingConfig?.display_name}」配置吗？删除后无法恢复。`}
        onConfirm={() => deletingConfig && deleteMutation.mutate(deletingConfig.id)}
        onCancel={() => setDeletingConfig(null)}
      />
    </div>
  );
};
