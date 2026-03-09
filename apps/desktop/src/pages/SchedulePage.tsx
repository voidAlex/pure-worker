/**
 * 课表日程页面组件
 * 展示班级日程列表，支持创建、编辑、删除日程事件
 */

import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands, ScheduleEvent, CreateScheduleEventInput, UpdateScheduleEventInput } from '@/bindings';
import { useToast } from '@/hooks/useToast';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import { EmptyState } from '@/components/shared/EmptyState';
import { Plus, Edit2, Trash2, CalendarDays, Clock, Filter, FileText } from 'lucide-react';

export const SchedulePage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();
  const [selectedClassId, setSelectedClassId] = useState<string>('');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingEvent, setEditingEvent] = useState<ScheduleEvent | null>(null);
  const [deletingEvent, setDeletingEvent] = useState<ScheduleEvent | null>(null);

  const [formData, setFormData] = useState<CreateScheduleEventInput>({
    class_id: '',
    title: '',
    start_at: '',
    end_at: null,
    linked_file_id: null,
  });

  const { data: classes } = useQuery({
    queryKey: ['classrooms'],
    queryFn: async () => {
      const result = await commands.listClassrooms();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const { data: events, isLoading } = useQuery({
    queryKey: ['scheduleEvents', selectedClassId],
    queryFn: async () => {
      if (!selectedClassId) return [];
      const result = await commands.listScheduleEvents({ class_id: selectedClassId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedClassId,
  });

  // 获取班级文件列表（用于文件关联选择）
  const { data: scheduleFiles } = useQuery({
    queryKey: ['scheduleFiles', selectedClassId],
    queryFn: async () => {
      if (!selectedClassId) return [];
      const result = await commands.listScheduleFiles({ class_id: selectedClassId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedClassId,
  });
  const createMutation = useMutation({
    mutationFn: async (input: CreateScheduleEventInput) => {
      const result = await commands.createScheduleEvent(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scheduleEvents', selectedClassId] });
      success('日程创建成功');
      setIsModalOpen(false);
      resetForm();
    },
    onError: (err) => error(`创建失败: ${err.message}`),
  });

  const updateMutation = useMutation({
    mutationFn: async (input: UpdateScheduleEventInput) => {
      const result = await commands.updateScheduleEvent(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scheduleEvents', selectedClassId] });
      success('日程更新成功');
      setIsModalOpen(false);
      resetForm();
    },
    onError: (err) => error(`更新失败: ${err.message}`),
  });

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.deleteScheduleEvent({ id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scheduleEvents', selectedClassId] });
      success('日程删除成功');
      setDeletingEvent(null);
    },
    onError: (err) => error(`删除失败: ${err.message}`),
  });

  const resetForm = () => {
    setFormData({
      class_id: selectedClassId,
      title: '',
      start_at: '',
      end_at: null,
      linked_file_id: null,
    });
    setEditingEvent(null);
  };

  const handleOpenModal = (event?: ScheduleEvent) => {
    if (event) {
      setEditingEvent(event);
      setFormData({
        class_id: event.class_id,
        title: event.title,
        start_at: event.start_at,
        end_at: event.end_at,
        linked_file_id: event.linked_file_id,
      });
    } else {
      resetForm();
    }
    setIsModalOpen(true);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (editingEvent) {
      updateMutation.mutate({
        id: editingEvent.id,
        title: formData.title,
        start_at: formData.start_at,
        end_at: formData.end_at,
        linked_file_id: formData.linked_file_id,
      });
    } else {
      createMutation.mutate(formData);
    }
  };

  const formatDateTime = (isoString: string) => {
    try {
      const date = new Date(isoString);
      return new Intl.DateTimeFormat('zh-CN', {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      }).format(date);
    } catch {
      return isoString;
    }
  };

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">课表日程</h1>
          <p className="text-sm text-gray-500 mt-1">管理班级课程与重要日程安排</p>
        </div>
        <button
          onClick={() => handleOpenModal()}
          disabled={!selectedClassId}
          className="flex items-center gap-2 px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Plus className="w-4 h-4" />
          新建日程
        </button>
      </header>

      <div className="flex items-center gap-4 bg-white p-4 rounded-xl shadow-sm border border-gray-100">
        <div className="flex items-center gap-2 text-gray-500">
          <Filter className="w-4 h-4" />
          <span className="text-sm font-medium">选择班级:</span>
        </div>
        <select
          value={selectedClassId}
          onChange={(e) => setSelectedClassId(e.target.value)}
          className="px-3 py-1.5 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 transition-shadow min-w-[200px]"
        >
          <option value="" disabled>请选择班级查看日程</option>
          {classes?.map((cls) => (
            <option key={cls.id} value={cls.id}>
              {cls.grade} {cls.class_name}
            </option>
          ))}
        </select>
      </div>

      <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        {!selectedClassId ? (
          <EmptyState
            icon={<CalendarDays className="w-8 h-8" />}
            title="请选择班级"
            description="选择一个班级以查看或管理其日程安排。"
          />
        ) : isLoading ? (
          <div className="p-8 text-center text-gray-500">加载中...</div>
        ) : events && events.length > 0 ? (
          <div className="p-6">
            <div className="space-y-4 relative before:absolute before:inset-0 before:ml-5 before:-translate-x-px md:before:mx-auto md:before:translate-x-0 before:h-full before:w-0.5 before:bg-gradient-to-b before:from-transparent before:via-gray-200 before:to-transparent">
              {events.map((event) => (
                <div key={event.id} className="relative flex items-center justify-between md:justify-normal md:odd:flex-row-reverse group is-active">
                  <div className="flex items-center justify-center w-10 h-10 rounded-full border-4 border-white bg-brand-100 text-brand-600 shadow shrink-0 md:order-1 md:group-odd:-translate-x-1/2 md:group-even:translate-x-1/2 z-10">
                    <Clock className="w-4 h-4" />
                  </div>
                  
                  <div className="w-[calc(100%-4rem)] md:w-[calc(50%-2.5rem)] p-4 rounded-xl border border-gray-100 bg-white shadow-sm hover:shadow-md transition-shadow">
                    <div className="flex items-center justify-between mb-1">
                      <time className="text-sm font-medium text-brand-600">
                        {formatDateTime(event.start_at)}
                        {event.end_at && ` - ${formatDateTime(event.end_at)}`}
                      </time>
                      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button
                          onClick={() => handleOpenModal(event)}
                          className="p-1.5 text-gray-400 hover:text-brand-600 hover:bg-brand-50 rounded-md transition-colors"
                          title="编辑"
                        >
                          <Edit2 className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() => setDeletingEvent(event)}
                          className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-md transition-colors"
                          title="删除"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                    <h3 className="text-lg font-semibold text-gray-900">{event.title}</h3>
                    {event.linked_file_id && (
                      <div className="flex items-center gap-1.5 mt-2 text-sm text-gray-500">
                        <FileText className="w-3.5 h-3.5" />
                        <span>已关联文件</span>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <EmptyState
            icon={<CalendarDays className="w-8 h-8" />}
            title="暂无日程"
            description="该班级暂无日程安排，点击右上角按钮新建日程。"
            action={
              <button
                onClick={() => handleOpenModal()}
                className="flex items-center gap-2 px-4 py-2 bg-brand-50 text-brand-700 rounded-lg hover:bg-brand-100 transition-colors font-medium"
              >
                <Plus className="w-4 h-4" />
                新建日程
              </button>
            }
          />
        )}
      </div>

      {/* Form Modal */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
          <div className="bg-white rounded-xl shadow-2xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-200">
            <div className="px-6 py-4 border-b border-gray-100 flex justify-between items-center">
              <h3 className="text-lg font-semibold text-gray-900">
                {editingEvent ? '编辑日程' : '新建日程'}
              </h3>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                &times;
              </button>
            </div>
            <form onSubmit={handleSubmit} className="p-6 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">日程标题</label>
                <input
                  type="text"
                  required
                  value={formData.title}
                  onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                  placeholder="例如：期中考试"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">开始时间</label>
                <input
                  type="datetime-local"
                  required
                  value={formData.start_at.slice(0, 16)}
                  onChange={(e) => setFormData({ ...formData, start_at: new Date(e.target.value).toISOString() })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">结束时间 (可选)</label>
                <input
                  type="datetime-local"
                  value={formData.end_at ? formData.end_at.slice(0, 16) : ''}
                  onChange={(e) => setFormData({ ...formData, end_at: e.target.value ? new Date(e.target.value).toISOString() : null })}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">关联文件 (可选)</label>
                <select
                  value={formData.linked_file_id || ''}
                  onChange={(e) => setFormData({ ...formData, linked_file_id: e.target.value || null })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow bg-white"
                >
                  <option value="">不关联文件</option>
                  {scheduleFiles?.map((file) => (
                    <option key={file.id} value={file.id}>
                      {file.file_name}
                    </option>
                  ))}
                </select>
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
        isOpen={!!deletingEvent}
        title="删除日程"
        message={`确定要删除日程 "${deletingEvent?.title}" 吗？此操作不可恢复。`}
        onConfirm={() => deletingEvent && deleteMutation.mutate(deletingEvent.id)}
        onCancel={() => setDeletingEvent(null)}
      />
    </div>
  );
};
