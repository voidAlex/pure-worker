/**
 * 班会活动页面组件
 * 支持班级选择、活动主题输入、AI生成文案、历史记录查看与采纳
 */

import React, { useState, useCallback, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands, type Classroom, type ActivityAnnouncement, type TemplateFile } from '@/services/commandClient';
import { useToast } from '@/hooks/useToast';
import { EmptyState } from '@/components/shared/EmptyState';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import {
  Megaphone,
  Sparkles,
  Check,
  X,
  Edit3,
  RefreshCw,
  ChevronDown,
  ChevronUp,
  Trash2,
} from 'lucide-react';

/** 通知对象映射 */
const AUDIENCE_MAP: Record<string, string> = {
  parent: '家长',
  student: '学生',
  internal: '全校',
};

/** 状态徽章颜色映射 */
const STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  draft: { bg: 'bg-yellow-50', text: 'text-yellow-700', label: '草稿' },
  adopted: { bg: 'bg-green-50', text: 'text-green-700', label: '已采纳' },
  rejected: { bg: 'bg-red-50', text: 'text-red-700', label: '已拒绝' },
};

export const ActivityAnnouncementsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  // 表单状态
  const [selectedClassId, setSelectedClassId] = useState<string>('');
  const [title, setTitle] = useState('');
  const [topic, setTopic] = useState('');
  const [audience, setAudience] = useState<string>('parent');
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>('');

  // 草稿状态
  const [currentDraftId, setCurrentDraftId] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [editingDraftId, setEditingDraftId] = useState<string | null>(null);
  const [editingDraftText, setEditingDraftText] = useState('');

  // 历史记录状态
  const [expandedHistoryIds, setExpandedHistoryIds] = useState<Set<string>>(new Set());
  const [deletingAnnouncementId, setDeletingAnnouncementId] = useState<string | null>(null);

  /** 查询班级列表 */
  const { data: classrooms, isLoading: isLoadingClasses } = useQuery({
    queryKey: ['classrooms'],
    queryFn: async () => {
      const result = await commands.listClassrooms();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  /** 查询校本模板列表 */
  const { data: templates } = useQuery({
    queryKey: ['templateFiles', 'activity_announcement'],
    queryFn: async () => {
      const result = await commands.listTemplateFiles({ type: 'activity_announcement', enabled: 1 });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  /** 查询活动通知列表 */
  const { data: announcements, isLoading: isLoadingAnnouncements } = useQuery({
    queryKey: ['activityAnnouncements', selectedClassId],
    queryFn: async () => {
      if (!selectedClassId) return [];
      const result = await commands.listActivityAnnouncements({
        class_id: selectedClassId,
        audience: null,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedClassId,
  });

  /** 生成文案 */
  const generateMutation = useMutation({
    mutationFn: async () => {
      const result = await commands.generateActivityAnnouncement({
        class_id: selectedClassId,
        title: title.trim(),
        topic: topic.trim() || null,
        audience,
        template_id: selectedTemplateId || null,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['activityAnnouncements', selectedClassId] });
      setCurrentDraftId(data.id);
      success('文案生成成功');
      setIsGenerating(false);
    },
    onError: (err) => {
      error(`生成失败: ${err.message}`);
      setIsGenerating(false);
    },
  });

  /** 更新文案 */
  const updateMutation = useMutation({
    mutationFn: async (input: {
      id: string;
      title: string | null;
      topic: string | null;
      audience: string | null;
      draft: string | null;
      adopted_text: string | null;
      template_id: string | null;
      status: string | null;
    }) => {
      const result = await commands.updateActivityAnnouncement(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['activityAnnouncements', selectedClassId] });
    },
    onError: (err) => error(`更新失败: ${err.message}`),
  });

  /** 删除文案 */
  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.deleteActivityAnnouncement({ id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['activityAnnouncements', selectedClassId] });
      success('删除成功');
      if (currentDraftId === deletingAnnouncementId) {
        setCurrentDraftId(null);
      }
      setDeletingAnnouncementId(null);
    },
    onError: (err) => error(`删除失败: ${err.message}`),
  });

  /** 触发生成 */
  const handleGenerate = useCallback(() => {
    if (!selectedClassId || !title.trim()) return;
    setIsGenerating(true);
    generateMutation.mutate();
  }, [selectedClassId, title, generateMutation]);

  /** 重新生成 */
  const handleRegenerate = useCallback(() => {
    setIsGenerating(true);
    generateMutation.mutate();
  }, [generateMutation]);

  /** 采纳文案 */
  const handleAdopt = useCallback(
    (announcement: ActivityAnnouncement) => {
      updateMutation.mutate({
        id: announcement.id,
        title: null,
        topic: null,
        audience: null,
        draft: null,
        adopted_text: announcement.draft,
        template_id: null,
        status: 'adopted',
      });
      success('文案已采纳');
    },
    [updateMutation, success],
  );

  /** 保存编辑 */
  const handleSaveEdit = useCallback(
    (id: string) => {
      updateMutation.mutate({
        id,
        title: null,
        topic: null,
        audience: null,
        draft: editingDraftText,
        adopted_text: null,
        template_id: null,
        status: null,
      });
      setEditingDraftId(null);
      success('文案已更新');
    },
    [updateMutation, editingDraftText, success],
  );

  /** 切换历史记录展开状态 */
  const toggleHistoryExpand = useCallback((id: string) => {
    setExpandedHistoryIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  /** 当前草稿 */
  const currentDraft = useMemo(() => {
    if (!announcements || !currentDraftId) return null;
    return announcements.find((a) => a.id === currentDraftId) || null;
  }, [announcements, currentDraftId]);

  /** 历史记录列表 */
  const historyList = useMemo(() => {
    if (!announcements) return [];
    return announcements.filter((a) => a.id !== currentDraftId);
  }, [announcements, currentDraftId]);

  return (
    <div className="space-y-6">
      {/* 页面标题 */}
      <header>
        <h1 className="text-2xl font-bold text-gray-900">班会活动</h1>
        <p className="text-sm text-gray-500 mt-1">AI 辅助生成班会与活动通知文案</p>
      </header>

      {/* 生成表单区域 */}
      <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">选择班级</label>
            <select
              value={selectedClassId}
              onChange={(e) => setSelectedClassId(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow bg-white"
              disabled={isGenerating}
            >
              <option value="">{isLoadingClasses ? '加载中...' : '请选择班级'}</option>
              {classrooms?.map((cls: Classroom) => (
                <option key={cls.id} value={cls.id}>
                  {cls.grade} {cls.class_name} - {cls.subject}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">活动标题</label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="例如：春季运动会通知"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
              disabled={isGenerating}
            />
          </div>
          <div className="md:col-span-2">
            <label className="block text-sm font-medium text-gray-700 mb-1">活动主题 (可选)</label>
            <input
              type="text"
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
              placeholder="例如：关于运动会报名与注意事项"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
              disabled={isGenerating}
            />
          </div>
        </div>
        {/* 校本模板选择 */}
        <div className="mt-4">
          <label className="block text-sm font-medium text-gray-700 mb-1">校本模板 (可选)</label>
          <select
            value={selectedTemplateId}
            onChange={(e) => setSelectedTemplateId(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow bg-white"
            disabled={isGenerating}
          >
            <option value="">不使用模板</option>
            {templates?.map((tpl: TemplateFile) => (
              <option key={tpl.id} value={tpl.id}>
                {tpl.school_scope ? `${tpl.school_scope} - ` : ''}{tpl.file_path}{tpl.version ? ` (${tpl.version})` : ''}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-wrap items-center justify-between gap-4 pt-4 border-t border-gray-100">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">通知对象</label>
            <div className="flex gap-4">
              {Object.entries(AUDIENCE_MAP).map(([key, label]) => (
                <label key={key} className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
                  <input
                    type="radio"
                    name="audience"
                    value={key}
                    checked={audience === key}
                    onChange={(e) => setAudience(e.target.value)}
                    className="text-brand-600 focus:ring-brand-500"
                    disabled={isGenerating}
                  />
                  {label}
                </label>
              ))}
            </div>
          </div>
          <button
            onClick={handleGenerate}
            disabled={!selectedClassId || !title.trim() || isGenerating}
            className="flex items-center gap-2 px-5 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isGenerating ? <RefreshCw className="w-4 h-4 animate-spin" /> : <Sparkles className="w-4 h-4" />}
            {isGenerating ? '生成中...' : 'AI 生成文案'}
          </button>
        </div>
      </div>

      {/* 当前草稿区域 */}
      {currentDraft && (
        <div className="bg-white rounded-xl shadow-sm border border-brand-200 overflow-hidden relative">
          <div className="absolute top-0 right-0 bg-brand-50 text-brand-600 text-xs px-2 py-1 rounded-bl-lg flex items-center gap-1 font-medium">
            <Sparkles className="w-3 h-3" />
            最新生成
          </div>
          <div className="px-6 py-4 border-b border-gray-50 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="font-medium text-gray-900">{currentDraft.title}</span>
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-blue-50 text-blue-700">
                {AUDIENCE_MAP[currentDraft.audience] || currentDraft.audience}
              </span>
              <span
                className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                  STATUS_STYLES[currentDraft.status]?.bg || 'bg-gray-100'
                } ${STATUS_STYLES[currentDraft.status]?.text || 'text-gray-700'}`}
              >
                {STATUS_STYLES[currentDraft.status]?.label || currentDraft.status}
              </span>
            </div>
          </div>
          <div className="px-6 py-4">
            {editingDraftId === currentDraft.id ? (
              <div className="space-y-3">
                <textarea
                  value={editingDraftText}
                  onChange={(e) => setEditingDraftText(e.target.value)}
                  rows={8}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow resize-y text-sm leading-relaxed"
                />
                <div className="flex justify-end gap-2">
                  <button
                    onClick={() => setEditingDraftId(null)}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-600 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
                  >
                    <X className="w-3.5 h-3.5" />
                    取消
                  </button>
                  <button
                    onClick={() => handleSaveEdit(currentDraft.id)}
                    disabled={updateMutation.isPending}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-white bg-brand-600 rounded-lg hover:bg-brand-700 transition-colors disabled:opacity-50"
                  >
                    <Check className="w-3.5 h-3.5" />
                    保存
                  </button>
                </div>
              </div>
            ) : (
              <p className="text-sm text-gray-700 leading-relaxed whitespace-pre-wrap">
                {currentDraft.adopted_text || currentDraft.draft || '（暂无内容）'}
              </p>
            )}
          </div>
          {!editingDraftId && currentDraft.status === 'draft' && (
            <div className="px-6 py-3 border-t border-gray-50 flex items-center gap-2">
              <button
                onClick={() => handleAdopt(currentDraft)}
                disabled={updateMutation.isPending}
                className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-green-700 bg-green-50 rounded-lg hover:bg-green-100 transition-colors disabled:opacity-50"
              >
                <Check className="w-3.5 h-3.5" />
                采纳
              </button>
              <button
                onClick={() => {
                  setEditingDraftId(currentDraft.id);
                  setEditingDraftText(currentDraft.draft || '');
                }}
                className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-gray-600 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors"
              >
                <Edit3 className="w-3.5 h-3.5" />
                编辑
              </button>
              <button
                onClick={handleRegenerate}
                disabled={isGenerating}
                className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-gray-600 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors ml-auto"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${isGenerating ? 'animate-spin' : ''}`} />
                重新生成
              </button>
            </div>
          )}
        </div>
      )}

      {/* 历史记录区域 */}
      {selectedClassId && (
        <div className="space-y-4">
          <h3 className="text-lg font-medium text-gray-900">历史记录</h3>
          {isLoadingAnnouncements ? (
            <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-8 text-center text-gray-500">
              加载记录中...
            </div>
          ) : historyList.length > 0 ? (
            <div className="space-y-3">
              {historyList.map((announcement) => {
                const isExpanded = expandedHistoryIds.has(announcement.id);
                return (
                  <div
                    key={announcement.id}
                    className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden"
                  >
                    <div
                      className="px-6 py-4 flex items-center justify-between cursor-pointer hover:bg-gray-50 transition-colors"
                      onClick={() => toggleHistoryExpand(announcement.id)}
                    >
                      <div className="flex items-center gap-3">
                        <span className="font-medium text-gray-900">{announcement.title}</span>
                        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-blue-50 text-blue-700">
                          {AUDIENCE_MAP[announcement.audience] || announcement.audience}
                        </span>
                        <span
                          className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                            STATUS_STYLES[announcement.status]?.bg || 'bg-gray-100'
                          } ${STATUS_STYLES[announcement.status]?.text || 'text-gray-700'}`}
                        >
                          {STATUS_STYLES[announcement.status]?.label || announcement.status}
                        </span>
                      </div>
                      <div className="flex items-center gap-4">
                        <span className="text-sm text-gray-500">
                          {new Date(announcement.created_at).toLocaleString('zh-CN')}
                        </span>
                        {isExpanded ? (
                          <ChevronUp className="w-4 h-4 text-gray-400" />
                        ) : (
                          <ChevronDown className="w-4 h-4 text-gray-400" />
                        )}
                      </div>
                    </div>
                    {isExpanded && (
                      <div className="px-6 py-4 border-t border-gray-50 bg-gray-50/50">
                        <p className="text-sm text-gray-700 leading-relaxed whitespace-pre-wrap mb-4">
                          {announcement.adopted_text || announcement.draft || '（暂无内容）'}
                        </p>
                        <div className="flex justify-end">
                          <button
                            onClick={() => setDeletingAnnouncementId(announcement.id)}
                            className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-red-600 bg-red-50 rounded-lg hover:bg-red-100 transition-colors"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                            删除
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ) : (
            !currentDraft && (
              <EmptyState
                icon={<Megaphone className="w-8 h-8" />}
                title="暂无历史记录"
                description="该班级目前没有活动通知记录。"
              />
            )
          )}
        </div>
      )}

      {/* 未选择班级时的空状态 */}
      {!selectedClassId && (
        <EmptyState
          icon={<Megaphone className="w-8 h-8" />}
          title="开始生成活动通知"
          description="选择班级并输入活动信息，点击「AI 生成文案」按钮，AI 将为您生成专业的通知文案。"
        />
      )}

      {/* 删除确认对话框 */}
      <ConfirmDialog
        isOpen={!!deletingAnnouncementId}
        title="删除通知"
        message="确定要删除这条活动通知吗？此操作不可恢复。"
        confirmText="删除"
        onConfirm={() => deletingAnnouncementId && deleteMutation.mutate(deletingAnnouncementId)}
        onCancel={() => setDeletingAnnouncementId(null)}
        isDestructive={true}
      />
    </div>
  );
};
