/**
 * 期末评语批量生成页面组件
 * 支持班级选择、学期设定、AI批量生成、进度监控、逐条审阅与采纳
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  commands,
  type Classroom,
  type SemesterComment,
  type AsyncTask,
  type Student,
} from '@/services/commandClient';
import { useToast } from '@/hooks/useToast';
import { EmptyState } from '@/components/shared/EmptyState';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import {
  FileText,
  Sparkles,
  Check,
  X,
  Edit3,
  Copy,
  AlertTriangle,
  CheckCheck,
  Download,
} from 'lucide-react';

/** 状态徽章颜色映射 */
const STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  draft: { bg: 'bg-gray-100', text: 'text-gray-600', label: '草稿' },
  adopted: { bg: 'bg-green-100', text: 'text-green-700', label: '已采纳' },
  rejected: { bg: 'bg-red-100', text: 'text-red-600', label: '已拒绝' },
};

type BatchProgress = {
  total: number;
  completed: number;
  failed: number;
  current_student_name: string | null;
};

export const SemesterCommentsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  // 表单状态
  const [selectedClassId, setSelectedClassId] = useState<string | null>(null);
  const [term, setTerm] = useState('');

  // 批量任务状态
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [batchProgress, setBatchProgress] = useState<BatchProgress | null>(null);

  // 编辑状态
  const [editingCommentId, setEditingCommentId] = useState<string | null>(null);
  const [editingDraftText, setEditingDraftText] = useState('');

  // 全部采纳确认
  const [showBatchAdoptConfirm, setShowBatchAdoptConfirm] = useState(false);

  /** 查询班级列表 */
  const { data: classrooms, isLoading: isLoadingClasses } = useQuery({
    queryKey: ['classrooms'],
    queryFn: async () => {
      const result = await commands.listClassrooms();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  /** 查询班级学生列表，用于 student_id → 姓名映射 */
  const { data: students } = useQuery({
    queryKey: ['students', selectedClassId],
    queryFn: async () => {
      const result = await commands.listStudents({ class_id: selectedClassId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedClassId,
  });

  /** 学生 ID → 姓名映射表 */
  const studentMap = useMemo(() => {
    const map = new Map<string, Student>();
    if (students) {
      for (const s of students) {
        map.set(s.id, s);
      }
    }
    return map;
  }, [students]);

  /** 查询评语列表（基于 taskId） */
  const { data: semesterComments, isLoading: isLoadingComments } = useQuery({
    queryKey: ['semesterComments', currentTaskId],
    queryFn: async () => {
      const result = await commands.listSemesterComments({
        student_id: null,
        term: null,
        task_id: currentTaskId,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!currentTaskId,
  });

  /** 统计已采纳数量 */
  const adoptedCount = useMemo(() => {
    if (!semesterComments) return 0;
    return semesterComments.filter((c) => c.status === 'adopted').length;
  }, [semesterComments]);

  /** 轮询批量任务进度 */
  useEffect(() => {
    if (!currentTaskId || !isGenerating) return;

    const intervalId = setInterval(async () => {
      try {
        const result = await commands.getBatchTaskProgress(currentTaskId);
        if (result.status === 'error') {
          console.error('获取进度失败:', result.error);
          return;
        }
        const task: AsyncTask = result.data;

        // 解析进度 JSON
        if (task.progress_json) {
          try {
            const progress = JSON.parse(task.progress_json) as BatchProgress;
            setBatchProgress(progress);
          } catch {
            // 进度 JSON 解析失败，忽略
          }
        }

        // 任务完成或失败时停止轮询
        if (task.status === 'completed' || task.status === 'failed') {
          setIsGenerating(false);
          queryClient.invalidateQueries({ queryKey: ['semesterComments', currentTaskId] });
          if (task.status === 'completed') {
            success('批量评语生成完成');
          } else {
            error(`批量生成失败: ${task.error_message || '未知错误'}`);
          }
        }
      } catch (err) {
        console.error('轮询进度异常:', err);
      }
    }, 2000);

    return () => clearInterval(intervalId);
  }, [currentTaskId, isGenerating, queryClient, success, error]);

  /** 触发批量生成 */
  const handleStartBatch = useCallback(async () => {
    if (!selectedClassId || !term.trim()) return;
    setIsGenerating(true);
    setBatchProgress(null);
    try {
      const result = await commands.generateSemesterCommentsBatch({
        class_id: selectedClassId,
        term: term.trim(),
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      setCurrentTaskId(result.data.id);
      success('批量生成任务已启动');
    } catch (err) {
      setIsGenerating(false);
      error(`启动批量生成失败: ${err instanceof Error ? err.message : '未知错误'}`);
    }
  }, [selectedClassId, term, success, error]);

  /** 更新单条评语 */
  const updateMutation = useMutation({
    mutationFn: async (input: {
      id: string;
      draft: string | null;
      adopted_text: string | null;
      status: string | null;
      evidence_json: string | null;
      evidence_count: number | null;
    }) => {
      const result = await commands.updateSemesterComment(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['semesterComments', currentTaskId] });
    },
    onError: (err) => error(`操作失败: ${err.message}`),
  });

  /** 采纳单条评语 */
  const handleAdopt = useCallback(
    (comment: SemesterComment) => {
      updateMutation.mutate({
        id: comment.id,
        draft: null,
        adopted_text: comment.draft,
        status: 'adopted',
        evidence_json: null,
        evidence_count: null,
      });
      success('评语已采纳');
    },
    [updateMutation, success],
  );

  /** 拒绝单条评语 */
  const handleReject = useCallback(
    (comment: SemesterComment) => {
      updateMutation.mutate({
        id: comment.id,
        draft: null,
        adopted_text: null,
        status: 'rejected',
        evidence_json: null,
        evidence_count: null,
      });
      success('评语已拒绝');
    },
    [updateMutation, success],
  );

  /** 保存编辑后的评语草稿 */
  const handleSaveEdit = useCallback(
    (commentId: string) => {
      updateMutation.mutate({
        id: commentId,
        draft: editingDraftText,
        adopted_text: null,
        status: null,
        evidence_json: null,
        evidence_count: null,
      });
      setEditingCommentId(null);
      setEditingDraftText('');
      success('评语已更新');
    },
    [updateMutation, editingDraftText, success],
  );

  /** 全部采纳 */
  const batchAdoptMutation = useMutation({
    mutationFn: async () => {
      if (!currentTaskId) throw new Error('无任务 ID');
      const result = await commands.batchAdoptSemesterComments({ task_id: currentTaskId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['semesterComments', currentTaskId] });
      success('已全部采纳');
      setShowBatchAdoptConfirm(false);
    },
    onError: (err) => {
      error(`全部采纳失败: ${err.message}`);
      setShowBatchAdoptConfirm(false);
    },
  });

  /** 复制全部评语到剪贴板 */
  const handleCopyAll = useCallback(async () => {
    if (!semesterComments || semesterComments.length === 0) return;
    const texts = semesterComments
      .map((c) => {
        const studentName = studentMap.get(c.student_id)?.name || c.student_id;
        const text = c.adopted_text || c.draft || '';
        return `【${studentName}】\n${text}`;
      })
      .join('\n\n---\n\n');
    try {
      await navigator.clipboard.writeText(texts);
      success('已复制全部评语到剪贴板');
    } catch {
      error('复制失败，请手动复制');
    }
  }, [semesterComments, studentMap, success, error]);

  /** 导出评语到 Excel */
  const handleExport = useCallback(async () => {
    if (!currentTaskId) return;
    try {
      const fileName = `期末评语_${term || '未知学期'}_${Date.now()}.xlsx`;
      const result = await commands.exportSemesterComments({
        task_id: currentTaskId,
        term: term || null,
        file_path: fileName,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      success(`已导出 ${result.data.exported_count} 条评语到 ${result.data.file_path}`);
    } catch (err) {
      error(`导出失败: ${err instanceof Error ? err.message : '未知错误'}`);
    }
  }, [currentTaskId, term, success, error]);

  /** 获取学生姓名 */
  const getStudentName = useCallback(
    (studentId: string): string => {
      return studentMap.get(studentId)?.name || studentId;
    },
    [studentMap],
  );

  /** 计算进度百分比 */
  const progressPercent = useMemo(() => {
    if (!batchProgress || batchProgress.total === 0) return 0;
    return Math.round(
      ((batchProgress.completed + batchProgress.failed) / batchProgress.total) * 100,
    );
  }, [batchProgress]);

  return (
    <div className="space-y-6">
      {/* 页面标题 */}
      <header>
        <h1 className="text-2xl font-bold text-gray-900">期末评语</h1>
        <p className="text-sm text-gray-500 mt-1">批量生成与审阅期末评语</p>
      </header>

      {/* 生成表单区域 */}
      <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
        <div className="flex flex-wrap items-end gap-4">
          {/* 班级选择 */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">选择班级</label>
            <select
              value={selectedClassId || ''}
              onChange={(e) => setSelectedClassId(e.target.value || null)}
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

          {/* 学期输入 */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">学期</label>
            <input
              type="text"
              value={term}
              onChange={(e) => setTerm(e.target.value)}
              placeholder="例如：2025-春季"
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
              disabled={isGenerating}
            />
          </div>

          {/* 生成按钮 */}
          <button
            onClick={handleStartBatch}
            disabled={!selectedClassId || !term.trim() || isGenerating}
            className="flex items-center gap-2 px-5 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Sparkles className="w-4 h-4" />
            {isGenerating ? '生成中...' : '批量生成 评语'}
          </button>
        </div>
      </div>

      {/* 进度条区域 */}
      {(isGenerating || batchProgress) && (
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
          <div className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-gray-700 font-medium">
                {isGenerating ? '正在生成评语...' : '生成完成'}
              </span>
              <span className="text-gray-500">{progressPercent}%</span>
            </div>

            {/* 进度条 */}
            <div className="w-full bg-gray-200 rounded-full h-2.5 overflow-hidden">
              <div
                className={`h-full rounded-full transition-all duration-500 ${
                  isGenerating ? 'bg-brand-600' : 'bg-green-500'
                }`}
                style={{ width: `${progressPercent}%` }}
              />
            </div>

            {/* 进度详情 */}
            {batchProgress && (
              <div className="flex items-center gap-4 text-sm text-gray-500">
                {batchProgress.current_student_name && isGenerating && (
                  <span>
                    正在生成:{' '}
                    <span className="font-medium text-gray-700">
                      {batchProgress.current_student_name}
                    </span>
                  </span>
                )}
                <span>
                  已完成 {batchProgress.completed}/{batchProgress.total}
                </span>
                {batchProgress.failed > 0 && (
                  <span className="flex items-center gap-1 text-amber-600">
                    <AlertTriangle className="w-3.5 h-3.5" />
                    {batchProgress.failed} 个生成失败
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      {/* 批量操作栏 */}
      {semesterComments && semesterComments.length > 0 && (
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 px-6 py-4 flex items-center justify-between">
          <span className="text-sm text-gray-600">
            共 <span className="font-semibold text-gray-900">{semesterComments.length}</span>{' '}
            条评语， 已采纳 <span className="font-semibold text-green-600">{adoptedCount}</span> 条
          </span>
          <div className="flex items-center gap-3">
            <button
              onClick={handleCopyAll}
              className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
            >
              <Copy className="w-4 h-4" />
              复制全部评语
            </button>
            <button
              onClick={handleExport}
              disabled={
                !semesterComments ||
                semesterComments.filter((c) => c.status === 'adopted').length === 0
              }
              className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Download className="w-4 h-4" />
              导出评语
            </button>
            <button
              onClick={() => setShowBatchAdoptConfirm(true)}
              disabled={batchAdoptMutation.isPending || adoptedCount === semesterComments.length}
              className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-lg hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <CheckCheck className="w-4 h-4" />
              {batchAdoptMutation.isPending ? '处理中...' : '全部采纳'}
            </button>
          </div>
        </div>
      )}

      {/* 评语卡片列表 */}
      {currentTaskId && (
        <div className="space-y-4">
          {isLoadingComments ? (
            <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-8 text-center text-gray-500">
              加载评语中...
            </div>
          ) : semesterComments && semesterComments.length > 0 ? (
            semesterComments.map((comment: SemesterComment) => {
              const studentName = getStudentName(comment.student_id);
              const statusStyle = STATUS_STYLES[comment.status] || STATUS_STYLES.draft;
              const isEditing = editingCommentId === comment.id;

              return (
                <div
                  key={comment.id}
                  className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden"
                >
                  {/* 卡片头部 */}
                  <div className="px-6 py-4 border-b border-gray-50 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <span className="font-medium text-gray-900">{studentName}</span>
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${statusStyle.bg} ${statusStyle.text}`}
                      >
                        {statusStyle.label}
                      </span>
                      {comment.evidence_count > 0 && (
                        <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-blue-50 text-blue-600">
                          <FileText className="w-3 h-3 mr-1" />
                          {comment.evidence_count} 条素材
                        </span>
                      )}
                    </div>
                  </div>

                  {/* 卡片内容 */}
                  <div className="px-6 py-4">
                    {isEditing ? (
                      <div className="space-y-3">
                        <textarea
                          value={editingDraftText}
                          onChange={(e) => setEditingDraftText(e.target.value)}
                          rows={6}
                          className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow resize-y text-sm leading-relaxed"
                        />
                        <div className="flex justify-end gap-2">
                          <button
                            onClick={() => {
                              setEditingCommentId(null);
                              setEditingDraftText('');
                            }}
                            className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-gray-600 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
                          >
                            <X className="w-3.5 h-3.5" />
                            取消
                          </button>
                          <button
                            onClick={() => handleSaveEdit(comment.id)}
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
                        {comment.adopted_text || comment.draft || '（暂无内容）'}
                      </p>
                    )}
                  </div>

                  {/* 卡片操作 */}
                  {!isEditing && comment.status === 'draft' && (
                    <div className="px-6 py-3 border-t border-gray-50 flex items-center gap-2">
                      <button
                        onClick={() => handleAdopt(comment)}
                        disabled={updateMutation.isPending}
                        className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-green-700 bg-green-50 rounded-lg hover:bg-green-100 transition-colors disabled:opacity-50"
                      >
                        <Check className="w-3.5 h-3.5" />
                        采纳
                      </button>
                      <button
                        onClick={() => handleReject(comment)}
                        disabled={updateMutation.isPending}
                        className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-red-600 bg-red-50 rounded-lg hover:bg-red-100 transition-colors disabled:opacity-50"
                      >
                        <X className="w-3.5 h-3.5" />
                        拒绝
                      </button>
                      <button
                        onClick={() => {
                          setEditingCommentId(comment.id);
                          setEditingDraftText(comment.draft || '');
                        }}
                        className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-gray-600 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors"
                      >
                        <Edit3 className="w-3.5 h-3.5" />
                        编辑
                      </button>
                    </div>
                  )}
                </div>
              );
            })
          ) : (
            !isGenerating && (
              <EmptyState
                icon={<FileText className="w-8 h-8" />}
                title="暂无评语"
                description="选择班级和学期后，点击「批量生成 评语」开始生成。"
              />
            )
          )}
        </div>
      )}

      {/* 未选择任务时的空状态 */}
      {!currentTaskId && !isGenerating && (
        <EmptyState
          icon={<FileText className="w-8 h-8" />}
          title="开始生成期末评语"
          description="选择班级并输入学期信息，点击「批量生成 评语」按钮，AI 将为每位学生生成个性化评语。"
        />
      )}

      {/* 全部采纳确认对话框 */}
      <ConfirmDialog
        isOpen={showBatchAdoptConfirm}
        title="全部采纳"
        message={`确定要采纳所有草稿评语吗？共 ${(semesterComments?.length || 0) - adoptedCount} 条待采纳。`}
        confirmText="全部采纳"
        onConfirm={() => batchAdoptMutation.mutate()}
        onCancel={() => setShowBatchAdoptConfirm(false)}
        isDestructive={false}
      />
    </div>
  );
};
