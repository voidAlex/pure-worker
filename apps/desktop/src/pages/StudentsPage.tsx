/**
 * 学生档案列表页面组件
 * 展示学生列表，支持按班级筛选，点击可进入详情页
 */

import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { commands, CreateStudentInput } from '@/services/commandClient';
import { useToast } from '@/hooks/useToast';
import { EmptyState } from '@/components/shared/EmptyState';
import { Plus, Search, GraduationCap, Filter } from 'lucide-react';

export const StudentsPage: React.FC = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { success, error } = useToast();
  const [selectedClassId, setSelectedClassId] = useState<string>('');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [formData, setFormData] = useState<CreateStudentInput>({
    student_no: '',
    name: '',
    gender: '男',
    class_id: '',
    meta_json: null,
  });

  const { data: classes } = useQuery({
    queryKey: ['classrooms'],
    queryFn: async () => {
      const result = await commands.listClassrooms();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const { data: students, isLoading } = useQuery({
    queryKey: ['students', selectedClassId],
    queryFn: async () => {
      const result = await commands.listStudents({ class_id: selectedClassId || null });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const createMutation = useMutation({
    mutationFn: async (input: CreateStudentInput) => {
      const result = await commands.createStudent(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['students'] });
      success('学生创建成功');
      setIsModalOpen(false);
      setFormData({ student_no: '', name: '', gender: '男', class_id: selectedClassId, meta_json: null });
    },
    onError: (err) => error(`创建失败: ${err.message}`),
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    createMutation.mutate(formData);
  };

  const getClassName = (classId: string) => {
    const cls = classes?.find(c => c.id === classId);
    return cls ? `${cls.grade} ${cls.class_name}` : '未知班级';
  };

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">学生档案</h1>
          <p className="text-sm text-gray-500 mt-1">管理学生基本信息与档案记录</p>
        </div>
        <button
          onClick={() => {
            setFormData(prev => ({ ...prev, class_id: selectedClassId || (classes?.[0]?.id || '') }));
            setIsModalOpen(true);
          }}
          className="flex items-center gap-2 px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium"
        >
          <Plus className="w-4 h-4" />
          新建学生
        </button>
      </header>

      <div className="flex items-center gap-4 bg-white p-4 rounded-xl shadow-sm border border-gray-100">
        <div className="flex items-center gap-2 text-gray-500">
          <Filter className="w-4 h-4" />
          <span className="text-sm font-medium">筛选:</span>
        </div>
        <select
          value={selectedClassId}
          onChange={(e) => setSelectedClassId(e.target.value)}
          className="px-3 py-1.5 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 transition-shadow"
        >
          <option value="">全部班级</option>
          {classes?.map((cls) => (
            <option key={cls.id} value={cls.id}>
              {cls.grade} {cls.class_name}
            </option>
          ))}
        </select>
        
        <div className="flex-1"></div>
        
        <div className="relative w-64">
          <input
            type="text"
            placeholder="搜索学生姓名或学号..."
            className="w-full pl-9 pr-4 py-1.5 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 transition-shadow"
          />
          <Search className="w-4 h-4 text-gray-400 absolute left-3 top-1/2 -translate-y-1/2" />
        </div>
      </div>

      <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        {isLoading ? (
          <div className="p-8 text-center text-gray-500">加载中...</div>
        ) : students && students.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 border-b border-gray-100 text-sm font-medium text-gray-500">
                  <th className="px-6 py-4">学号</th>
                  <th className="px-6 py-4">姓名</th>
                  <th className="px-6 py-4">性别</th>
                  <th className="px-6 py-4">班级</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {students.map((student) => (
                  <tr 
                    key={student.id} 
                    onClick={() => navigate(`/students/${student.id}`)}
                    className="hover:bg-brand-50/50 transition-colors cursor-pointer group"
                  >
                    <td className="px-6 py-4 text-gray-500 font-mono text-sm">{student.student_no}</td>
                    <td className="px-6 py-4 font-medium text-gray-900 group-hover:text-brand-600 transition-colors">
                      {student.name}
                    </td>
                    <td className="px-6 py-4 text-gray-600">
                      <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                        student.gender === '男' ? 'bg-blue-50 text-blue-700' : 
                        student.gender === '女' ? 'bg-pink-50 text-pink-700' : 
                        'bg-gray-100 text-gray-700'
                      }`}>
                        {student.gender || '未知'}
                      </span>
                    </td>
                    <td className="px-6 py-4 text-gray-600 text-sm">
                      {getClassName(student.class_id)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState
            icon={<GraduationCap className="w-8 h-8" />}
            title="暂无学生"
            description={selectedClassId ? "该班级下暂无学生数据" : "系统中暂无学生数据"}
          />
        )}
      </div>

      {/* Form Modal */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
          <div className="bg-white rounded-xl shadow-2xl w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-200">
            <div className="px-6 py-4 border-b border-gray-100 flex justify-between items-center">
              <h3 className="text-lg font-semibold text-gray-900">新建学生</h3>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                &times;
              </button>
            </div>
            <form onSubmit={handleSubmit} className="p-6 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">学号</label>
                <input
                  type="text"
                  required
                  value={formData.student_no}
                  onChange={(e) => setFormData({ ...formData, student_no: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                  placeholder="例如：20230101"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">姓名</label>
                <input
                  type="text"
                  required
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                  placeholder="例如：张三"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">性别</label>
                <select
                  value={formData.gender || ''}
                  onChange={(e) => setFormData({ ...formData, gender: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                >
                  <option value="男">男</option>
                  <option value="女">女</option>
                  <option value="">未知</option>
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">所属班级</label>
                <select
                  required
                  value={formData.class_id}
                  onChange={(e) => setFormData({ ...formData, class_id: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                >
                  <option value="" disabled>请选择班级</option>
                  {classes?.map((cls) => (
                    <option key={cls.id} value={cls.id}>
                      {cls.grade} {cls.class_name}
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
                  disabled={createMutation.isPending}
                  className="px-4 py-2 text-sm font-medium text-white bg-brand-600 rounded-lg hover:bg-brand-700 disabled:opacity-50"
                >
                  {createMutation.isPending ? '保存中...' : '保存'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
