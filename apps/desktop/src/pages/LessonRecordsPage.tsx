/**
 * 行课记录管理页面
 * 用于记录和管理每次行课的详细信息，包括教学主题、目标、作业摘要和教师备注。
 */

import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, Edit2, Trash2, Calendar, BookOpen } from 'lucide-react';
import { commands, type LessonRecord, type CreateLessonRecordInput, type AppError } from '@/services/commandClient';

/** 从 AppError 联合类型中提取错误信息字符串 */
const getErrorMessage = (err: AppError): string => {
  const values = Object.values(err as Record<string, string>);
  return values[0] ?? '未知错误';
};
import { useToast } from '@/hooks/useToast';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import { EmptyState } from '@/components/shared/EmptyState';

export const LessonRecordsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  const [showForm, setShowForm] = useState(false);
  const [editingRecord, setEditingRecord] = useState<LessonRecord | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<LessonRecord | null>(null);

  const [formData, setFormData] = useState<Partial<CreateLessonRecordInput>>({
    class_id: '',
    subject: '',
    lesson_date: new Date().toISOString().split('T')[0],
    topic: '',
    teaching_goal: '',
    homework_summary: '',
    teacher_note: '',
    status: 'planned',
  });

  const { data: records, isLoading } = useQuery({
    queryKey: ['lesson-records'],
    queryFn: async () => {
      const result = await commands.listLessonRecords({
        class_id: null,
        from_date: null,
        to_date: null,
        status: null,
      });
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error('获取行课记录失败');
    },
  });

  const createMutation = useMutation({
    mutationFn: async (input: CreateLessonRecordInput) => {
      const result = await commands.createLessonRecord(input);
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error(getErrorMessage(result.error));
    },
    onSuccess: () => {
      success('行课记录已创建');
      queryClient.invalidateQueries({ queryKey: ['lesson-records'] });
      setShowForm(false);
      resetForm();
    },
    onError: (err: Error) => error(err.message),
  });

  const updateMutation = useMutation({
    mutationFn: async ({ id, input }: { id: string; input: Partial<CreateLessonRecordInput> }) => {
      // Convert empty strings/undefined to null for API compatibility
      const sanitizedInput = {
        ...input,
        subject: input.subject?.trim() || null,
        lesson_date: input.lesson_date?.trim() || null,
        lesson_index: input.lesson_index ?? null,
        topic: input.topic?.trim() || null,
        teaching_goal: input.teaching_goal?.trim() || null,
        homework_summary: input.homework_summary?.trim() || null,
        teacher_note: input.teacher_note?.trim() || null,
        status: input.status || null,
      };
      const result = await commands.updateLessonRecord({ id, ...sanitizedInput });
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error(getErrorMessage(result.error));
    },
    onSuccess: () => {
      success('行课记录已更新');
      queryClient.invalidateQueries({ queryKey: ['lesson-records'] });
      setShowForm(false);
      setEditingRecord(null);
      resetForm();
    },
    onError: (err: Error) => error(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.deleteLessonRecord({ id });
      if (result.status === 'ok') {
        return result.data;
      }
      throw new Error(getErrorMessage(result.error));
    },
    onSuccess: () => {
      success('行课记录已删除');
      queryClient.invalidateQueries({ queryKey: ['lesson-records'] });
      setDeleteTarget(null);
    },
    onError: (err: Error) => error(err.message),
  });

  const resetForm = () => {
    setFormData({
      class_id: '',
      subject: '',
      lesson_date: new Date().toISOString().split('T')[0],
      topic: '',
      teaching_goal: '',
      homework_summary: '',
      teacher_note: '',
      status: 'planned',
    });
  };

  const openCreateForm = () => {
    setEditingRecord(null);
    resetForm();
    setShowForm(true);
  };

  const openEditForm = (record: LessonRecord) => {
    setEditingRecord(record);
    setFormData({
      class_id: record.class_id,
      subject: record.subject,
      lesson_date: record.lesson_date,
      topic: record.topic || '',
      teaching_goal: record.teaching_goal || '',
      homework_summary: record.homework_summary || '',
      teacher_note: record.teacher_note || '',
      status: record.status,
    });
    setShowForm(true);
  };

  const handleSubmit = () => {
    if (!formData.class_id || !formData.subject || !formData.lesson_date) {
      error('请填写班级、科目和日期');
      return;
    }

    const input: CreateLessonRecordInput = {
      class_id: formData.class_id!,
      subject: formData.subject!,
      lesson_date: formData.lesson_date!,
      schedule_event_id: null,
      lesson_index: null,
      topic: formData.topic || null,
      teaching_goal: formData.teaching_goal || null,
      homework_summary: formData.homework_summary || null,
      teacher_note: formData.teacher_note || null,
      status: formData.status || 'planned',
    };

    if (editingRecord) {
      updateMutation.mutate({ id: editingRecord.id, input });
    } else {
      createMutation.mutate(input);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed':
        return 'bg-green-100 text-green-700';
      case 'in_progress':
        return 'bg-blue-100 text-blue-700';
      case 'planned':
        return 'bg-gray-100 text-gray-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  };

  const getStatusLabel = (status: string) => {
    switch (status) {
      case 'completed':
        return '已完成';
      case 'in_progress':
        return '进行中';
      case 'planned':
        return '计划中';
      default:
        return status;
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">行课记录</h1>
          <p className="text-sm text-gray-500 mt-1">管理每次行课的详细信息和教学记录</p>
        </div>
        <button
          onClick={openCreateForm}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          <Plus size={18} />
          新增行课记录
        </button>
      </div>

      {showForm && (
        <div className="mb-6 p-4 bg-gray-50 rounded-lg border">
          <h3 className="font-medium mb-4">{editingRecord ? '编辑行课记录' : '新增行课记录'}</h3>
          <div className="grid grid-cols-2 gap-4 mb-4">
            <input
              type="text"
              placeholder="班级ID"
              className="px-3 py-2 border rounded-lg"
              value={formData.class_id || ''}
              onChange={(e) => setFormData({ ...formData, class_id: e.target.value })}
            />
            <input
              type="text"
              placeholder="科目"
              className="px-3 py-2 border rounded-lg"
              value={formData.subject || ''}
              onChange={(e) => setFormData({ ...formData, subject: e.target.value })}
            />
            <input
              type="date"
              placeholder="日期"
              className="px-3 py-2 border rounded-lg"
              value={formData.lesson_date || ''}
              onChange={(e) => setFormData({ ...formData, lesson_date: e.target.value })}
            />
            <select
              className="px-3 py-2 border rounded-lg"
              value={formData.status || ''}
              onChange={(e) => setFormData({ ...formData, status: e.target.value })}
            >
              <option value="planned">计划中</option>
              <option value="in_progress">进行中</option>
              <option value="completed">已完成</option>
            </select>
            <input
              type="text"
              placeholder="教学主题"
              className="col-span-2 px-3 py-2 border rounded-lg"
              value={formData.topic || ''}
              onChange={(e) => setFormData({ ...formData, topic: e.target.value })}
            />
            <textarea
              placeholder="教学目标"
              className="col-span-2 px-3 py-2 border rounded-lg"
              rows={2}
              value={formData.teaching_goal || ''}
              onChange={(e) => setFormData({ ...formData, teaching_goal: e.target.value })}
            />
            <textarea
              placeholder="作业摘要"
              className="col-span-2 px-3 py-2 border rounded-lg"
              rows={2}
              value={formData.homework_summary || ''}
              onChange={(e) => setFormData({ ...formData, homework_summary: e.target.value })}
            />
            <textarea
              placeholder="教师备注"
              className="col-span-2 px-3 py-2 border rounded-lg"
              rows={2}
              value={formData.teacher_note || ''}
              onChange={(e) => setFormData({ ...formData, teacher_note: e.target.value })}
            />
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleSubmit}
              disabled={createMutation.isPending || updateMutation.isPending}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300"
            >
              {createMutation.isPending || updateMutation.isPending ? '保存中...' : '保存'}
            </button>
            <button
              onClick={() => {
                setShowForm(false);
                setEditingRecord(null);
              }}
              className="px-4 py-2 border rounded-lg hover:bg-gray-100"
            >
              取消
            </button>
          </div>
        </div>
      )}

      {isLoading ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      ) : records && records.length > 0 ? (
        <div className="flex-1 overflow-y-auto space-y-3">
          {records.map((record) => (
            <div
              key={record.id}
              className="p-4 bg-white rounded-lg border hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <Calendar size={18} className="text-gray-400" />
                    <span className="font-medium">{record.lesson_date}</span>
                    <span className="text-gray-500">{record.subject}</span>
                    <span className={`px-2 py-0.5 rounded text-xs ${getStatusColor(record.status)}`}>
                      {getStatusLabel(record.status)}
                    </span>
                  </div>
                  {record.topic && (
                    <div className="flex items-center gap-2 mb-2">
                      <BookOpen size={16} className="text-gray-400" />
                      <span className="text-gray-700">{record.topic}</span>
                    </div>
                  )}
                  {record.teaching_goal && (
                    <p className="text-sm text-gray-600 mb-1">
                      <span className="font-medium">教学目标：</span>
                      {record.teaching_goal}
                    </p>
                  )}
                  {record.homework_summary && (
                    <p className="text-sm text-gray-600 mb-1">
                      <span className="font-medium">作业摘要：</span>
                      {record.homework_summary}
                    </p>
                  )}
                  {record.teacher_note && (
                    <p className="text-sm text-gray-500">
                      <span className="font-medium">教师备注：</span>
                      {record.teacher_note}
                    </p>
                  )}
                </div>
                <div className="flex gap-2 ml-4">
                  <button
                    onClick={() => openEditForm(record)}
                    className="p-1.5 text-gray-500 hover:bg-gray-100 rounded"
                    title="编辑"
                  >
                    <Edit2 size={16} />
                  </button>
                  <button
                    onClick={() => setDeleteTarget(record)}
                    className="p-1.5 text-red-500 hover:bg-red-50 rounded"
                    title="删除"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <EmptyState
          title="暂无行课记录"
          description="点击上方按钮创建第一条行课记录"
          icon={<BookOpen size={48} className="text-gray-400" />}
        />
      )}

      <ConfirmDialog
        isOpen={deleteTarget !== null}
        title="确认删除"
        message={`确定要删除这条行课记录吗？此操作不可恢复。`}
        confirmText="删除"
        isDestructive
        onConfirm={() => deleteTarget && deleteMutation.mutate(deleteTarget.id)}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
};
