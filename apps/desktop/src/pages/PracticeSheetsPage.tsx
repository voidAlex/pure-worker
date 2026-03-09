/**
 * 错题练习页面组件
 * 用于浏览学生的错题记录，并根据错题和知识点生成个性化练习卷
 */

import React, { useState, useCallback, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  commands,
  type WrongAnswerRecord,
  type PracticeSheet,
  type Student,
  type Classroom,
  type ListWrongAnswersInput,
  type ResolveWrongAnswerCommandInput,
  type GeneratePracticeSheetInput,
  type ListStudentPracticeSheetsInput,
  type DeletePracticeSheetInput,
} from '@/bindings';
import { useToast } from '@/hooks/useToast';
import { EmptyState } from '@/components/shared/EmptyState';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import {
  BookOpen,
  FileQuestion,
  Download,
  Trash2,
  Plus,
  Check,
  Filter,
  RefreshCcw,
} from 'lucide-react';

/** 练习卷状态徽章颜色映射 */
const SHEET_STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  pending: { bg: 'bg-gray-100', text: 'text-gray-600', label: '生成中' },
  completed: { bg: 'bg-green-100', text: 'text-green-700', label: '已完成' },
  failed: { bg: 'bg-red-100', text: 'text-red-600', label: '失败' },
};


export const PracticeSheetsPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  // 标签页状态
  const [activeTab, setActiveTab] = useState<'wrong-answers' | 'practice-sheets'>('wrong-answers');

  // 班级和学生选择状态
  const [selectedClassId, setSelectedClassId] = useState<string>('');
  const [selectedStudentId, setSelectedStudentId] = useState<string>('');

  // 错题过滤状态
  const [knowledgePointFilter, setKnowledgePointFilter] = useState<string>('');
  const [unresolvedOnly, setUnresolvedOnly] = useState<boolean>(true);

  // 练习卷生成表单状态
  const [sheetTitle, setSheetTitle] = useState<string>('');
  const [sheetKnowledgePoints, setSheetKnowledgePoints] = useState<string>('');
  const [sheetDifficulty, setSheetDifficulty] = useState<string>('medium');
  const [sheetQuestionCount, setSheetQuestionCount] = useState<number>(10);

  // 删除确认框状态
  const [deleteSheetId, setDeleteSheetId] = useState<string | null>(null);

  /** 获取班级列表 */
  const { data: classrooms = [] } = useQuery<Classroom[]>({
    queryKey: ['classrooms'],
    queryFn: async () => {
      const result = await commands.listClassrooms();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  /** 获取学生列表 */
  const { data: students = [] } = useQuery<Student[]>({
    queryKey: ['students', selectedClassId],
    queryFn: async () => {
      if (!selectedClassId) return [];
      const result = await commands.listStudents({ class_id: selectedClassId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedClassId,
  });

  /** 学生 ID 到名称的映射 */
  const studentNameMap = useMemo(() => {
    const map = new Map<string, string>();
    students.forEach((s) => map.set(s.id, s.name));
    return map;
  }, [students]);

  /** 获取错题列表 */
  const { data: wrongAnswers = [], isLoading: isLoadingWrongAnswers } = useQuery<WrongAnswerRecord[]>({
    queryKey: ['wrong-answers', selectedStudentId, knowledgePointFilter, unresolvedOnly],
    queryFn: async () => {
      const input: ListWrongAnswersInput = {
        student_id: selectedStudentId || null,
        job_id: null,
        knowledge_point: knowledgePointFilter || null,
        unresolved_only: unresolvedOnly,
        limit: 100,
      };
      const result = await commands.listWrongAnswers(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  /** 获取练习卷列表 */
  const { data: practiceSheets = [], isLoading: isLoadingSheets } = useQuery<PracticeSheet[]>({
    queryKey: ['practice-sheets', selectedStudentId],
    queryFn: async () => {
      if (!selectedStudentId) return [];
      const input: ListStudentPracticeSheetsInput = { student_id: selectedStudentId };
      const result = await commands.listStudentPracticeSheets(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedStudentId,
  });

  /** 解决错题的 Mutation */
  const resolveMutation = useMutation({
    mutationFn: async (input: ResolveWrongAnswerCommandInput) => {
      const result = await commands.resolveWrongAnswer(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      success('已标记为解决');
      queryClient.invalidateQueries({ queryKey: ['wrong-answers'] });
    },
    onError: (err) => {
      error(`操作失败: ${err.message}`);
    },
  });

  /** 生成练习卷的 Mutation */
  const generateSheetMutation = useMutation({
    mutationFn: async (input: GeneratePracticeSheetInput) => {
      const result = await commands.generatePracticeSheet(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      success('练习卷生成任务已提交');
      setSheetTitle('');
      setSheetKnowledgePoints('');
      queryClient.invalidateQueries({ queryKey: ['practice-sheets'] });
    },
    onError: (err) => {
      error(`生成失败: ${err.message}`);
    },
  });

  /** 删除练习卷的 Mutation */
  const deleteSheetMutation = useMutation({
    mutationFn: async (input: DeletePracticeSheetInput) => {
      const result = await commands.deletePracticeSheet(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      success('练习卷已删除');
      setDeleteSheetId(null);
      queryClient.invalidateQueries({ queryKey: ['practice-sheets'] });
    },
    onError: (err) => {
      error(`删除失败: ${err.message}`);
    },
  });

  /** 处理解决错题 */
  const handleResolve = useCallback((id: string) => {
    resolveMutation.mutate({ id });
  }, [resolveMutation]);

  /** 处理生成练习卷 */
  const handleGenerateSheet = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedStudentId) {
      error('请先选择学生');
      return;
    }
    if (!sheetTitle.trim()) {
      error('请输入练习卷标题');
      return;
    }

    const points = sheetKnowledgePoints
      .split(/[,，]/)
      .map((p) => p.trim())
      .filter(Boolean);

    generateSheetMutation.mutate({
      student_id: selectedStudentId,
      title: sheetTitle.trim(),
      knowledge_points: points.length > 0 ? points : null,
      difficulty: sheetDifficulty,
      question_count: sheetQuestionCount,
    });
  }, [selectedStudentId, sheetTitle, sheetKnowledgePoints, sheetDifficulty, sheetQuestionCount, generateSheetMutation, error]);

  /** 处理删除练习卷确认 */
  const handleConfirmDelete = useCallback(() => {
    if (deleteSheetId) {
      deleteSheetMutation.mutate({ id: deleteSheetId });
    }
  }, [deleteSheetId, deleteSheetMutation]);

  /** 处理取消删除 */
  const handleCancelDelete = useCallback(() => {
    setDeleteSheetId(null);
  }, []);

  /** 渲染错题列表 */
  const renderWrongAnswers = useCallback(() => {
    return (
      <div className="space-y-4">
        <div className="flex flex-wrap gap-4 items-end bg-white p-4 rounded-xl shadow-sm border border-gray-100">
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">知识点过滤</label>
            <div className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <Filter className="h-4 w-4 text-gray-400" />
              </div>
              <input
                type="text"
                value={knowledgePointFilter}
                onChange={(e) => setKnowledgePointFilter(e.target.value)}
                placeholder="输入知识点关键字..."
                className="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm"
              />
            </div>
          </div>
          <div className="flex items-center h-10">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={unresolvedOnly}
                onChange={(e) => setUnresolvedOnly(e.target.checked)}
                className="rounded border-gray-300 text-brand-600 focus:ring-brand-500 h-4 w-4"
              />
              <span className="text-sm text-gray-700">仅显示未解决</span>
            </label>
          </div>
        </div>

        <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
          {isLoadingWrongAnswers ? (
            <div className="p-8 text-center text-gray-500">
              <RefreshCcw className="w-6 h-6 animate-spin mx-auto mb-2" />
              <p>加载中...</p>
            </div>
          ) : wrongAnswers.length === 0 ? (
            <EmptyState
              icon={<BookOpen className="w-8 h-8" />}
              title="暂无错题记录"
              description="当前过滤条件下没有找到错题记录"
            />
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">学生</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">题号</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">知识点</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">学生答案</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">正确答案</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">得分</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">难度</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">错误类型</th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">状态</th>
                    <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">操作</th>
                  </tr>
                </thead>
                <tbody className="bg-white divide-y divide-gray-200">
                  {wrongAnswers.map((record) => (
                    <tr key={record.id} className="hover:bg-gray-50">
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                        {studentNameMap.get(record.student_id) || '未知'}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{record.question_no}</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{record.knowledge_point || '-'}</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-red-600">{record.student_answer || '-'}</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-green-600">{record.correct_answer || '-'}</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                        {record.score !== null ? record.score : '-'}/{record.full_score !== null ? record.full_score : '-'}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{record.difficulty || '-'}</td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{record.error_type || '-'}</td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                          record.is_resolved ? 'bg-green-100 text-green-800' : 'bg-yellow-100 text-yellow-800'
                        }`}>
                          {record.is_resolved ? '已解决' : '未解决'}
                        </span>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                        {!record.is_resolved && (
                          <button
                            onClick={() => handleResolve(record.id)}
                            disabled={resolveMutation.isPending}
                            className="text-brand-600 hover:text-brand-900 flex items-center justify-end gap-1 ml-auto"
                          >
                            <Check className="w-4 h-4" />
                            <span>解决</span>
                          </button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    );
  }, [knowledgePointFilter, unresolvedOnly, isLoadingWrongAnswers, wrongAnswers, studentNameMap, resolveMutation.isPending, handleResolve]);

  /** 渲染练习卷列表 */
  const renderPracticeSheets = useCallback(() => {
    if (!selectedStudentId) {
      return (
        <EmptyState
          icon={<FileQuestion className="w-8 h-8" />}
          title="请先选择学生"
          description="选择学生后即可查看和生成个性化练习卷"
        />
      );
    }

    return (
      <div className="space-y-6">
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
          <h3 className="text-lg font-medium text-gray-900 mb-4">生成新练习卷</h3>
          <form onSubmit={handleGenerateSheet} className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">练习卷标题</label>
                <input
                  type="text"
                  value={sheetTitle}
                  onChange={(e) => setSheetTitle(e.target.value)}
                  placeholder="例如：期中复习专项练习"
                  className="block w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">知识点（逗号分隔）</label>
                <input
                  type="text"
                  value={sheetKnowledgePoints}
                  onChange={(e) => setSheetKnowledgePoints(e.target.value)}
                  placeholder="例如：一元二次方程，勾股定理"
                  className="block w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">难度</label>
                <select
                  value={sheetDifficulty}
                  onChange={(e) => setSheetDifficulty(e.target.value)}
                  className="block w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm"
                >
                  <option value="easy">简单</option>
                  <option value="medium">中等</option>
                  <option value="hard">困难</option>
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">题目数量</label>
                <input
                  type="number"
                  min="1"
                  max="50"
                  value={sheetQuestionCount}
                  onChange={(e) => setSheetQuestionCount(parseInt(e.target.value) || 10)}
                  className="block w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm"
                />
              </div>
            </div>
            <div className="flex justify-end">
              <button
                type="submit"
                disabled={generateSheetMutation.isPending}
                className="flex items-center gap-2 px-5 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {generateSheetMutation.isPending ? (
                  <RefreshCcw className="w-4 h-4 animate-spin" />
                ) : (
                  <Plus className="w-4 h-4" />
                )}
                <span>生成练习卷</span>
              </button>
            </div>
          </form>
        </div>

        <div className="space-y-4">
          <h3 className="text-lg font-medium text-gray-900">历史练习卷</h3>
          {isLoadingSheets ? (
            <div className="p-8 text-center text-gray-500">
              <RefreshCcw className="w-6 h-6 animate-spin mx-auto mb-2" />
              <p>加载中...</p>
            </div>
          ) : practiceSheets.length === 0 ? (
            <EmptyState
              icon={<FileQuestion className="w-8 h-8" />}
              title="暂无练习卷"
              description="该学生还没有生成过练习卷"
            />
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {practiceSheets.map((sheet) => {
                const statusStyle = SHEET_STATUS_STYLES[sheet.status] || SHEET_STATUS_STYLES.pending;
                return (
                  <div key={sheet.id} className="bg-white rounded-xl shadow-sm border border-gray-100 p-5 flex flex-col">
                    <div className="flex justify-between items-start mb-3">
                      <h4 className="text-base font-medium text-gray-900 line-clamp-2" title={sheet.title}>
                        {sheet.title}
                      </h4>
                      <span className={`shrink-0 ml-2 px-2.5 py-0.5 rounded-full text-xs font-medium ${statusStyle.bg} ${statusStyle.text}`}>
                        {statusStyle.label}
                      </span>
                    </div>
                    <div className="text-sm text-gray-500 space-y-1 mb-4 flex-1">
                      <p>题目数量：{sheet.question_count}题</p>
                      <p>难度：{sheet.difficulty === 'easy' ? '简单' : sheet.difficulty === 'hard' ? '困难' : '中等'}</p>
                      <p>创建时间：{new Date(sheet.created_at).toLocaleDateString()}</p>
                    </div>
                    <div className="flex items-center justify-between pt-3 border-t border-gray-100">
                      {sheet.file_path ? (
                        <a
                          href={`file://${sheet.file_path}`}
                          target="_blank"
                          rel="noreferrer"
                          className="flex items-center gap-1 text-sm text-brand-600 hover:text-brand-700 font-medium"
                        >
                          <Download className="w-4 h-4" />
                          <span>下载试卷</span>
                        </a>
                      ) : (
                        <span className="text-sm text-gray-400">暂无文件</span>
                      )}
                      <button
                        onClick={() => setDeleteSheetId(sheet.id)}
                        className="flex items-center gap-1 p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                        title="删除"
                      >
                        <Trash2 className="w-4 h-4" />
                        <span className="text-sm">删除</span>
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    );
  }, [
    selectedStudentId,
    sheetTitle,
    sheetKnowledgePoints,
    sheetDifficulty,
    sheetQuestionCount,
    generateSheetMutation.isPending,
    isLoadingSheets,
    practiceSheets,
    handleGenerateSheet,
  ]);

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-bold text-gray-900">错题练习</h1>
        <p className="text-sm text-gray-500 mt-1">浏览错题记录，生成个性化练习卷</p>
      </header>

      <div className="bg-white p-4 rounded-xl shadow-sm border border-gray-100 flex flex-wrap gap-4">
        <div className="w-64">
          <label className="block text-sm font-medium text-gray-700 mb-1">选择班级</label>
          <select
            value={selectedClassId}
            onChange={(e) => {
              setSelectedClassId(e.target.value);
              setSelectedStudentId('');
            }}
            className="block w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm"
          >
            <option value="">-- 请选择班级 --</option>
            {classrooms.map((c) => (
              <option key={c.id} value={c.id}>
                {c.grade} {c.class_name}
              </option>
            ))}
          </select>
        </div>
        <div className="w-64">
          <label className="block text-sm font-medium text-gray-700 mb-1">选择学生（可选）</label>
          <select
            value={selectedStudentId}
            onChange={(e) => setSelectedStudentId(e.target.value)}
            disabled={!selectedClassId}
            className="block w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-brand-500 focus:border-brand-500 sm:text-sm disabled:bg-gray-50 disabled:text-gray-500"
          >
            <option value="">-- 所有学生 --</option>
            {students.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name} ({s.student_no})
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div className="flex border-b border-gray-200">
          <button
            onClick={() => setActiveTab('wrong-answers')}
            className={`flex-1 py-4 px-6 text-center text-sm font-medium transition-colors ${
              activeTab === 'wrong-answers'
                ? 'border-b-2 border-brand-600 text-brand-700'
                : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
            }`}
          >
            <div className="flex items-center justify-center gap-2">
              <BookOpen className="w-4 h-4" />
              <span>错题浏览</span>
            </div>
          </button>
          <button
            onClick={() => setActiveTab('practice-sheets')}
            className={`flex-1 py-4 px-6 text-center text-sm font-medium transition-colors ${
              activeTab === 'practice-sheets'
                ? 'border-b-2 border-brand-600 text-brand-700'
                : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
            }`}
          >
            <div className="flex items-center justify-center gap-2">
              <FileQuestion className="w-4 h-4" />
              <span>练习卷</span>
            </div>
          </button>
        </div>

        <div className="p-6">
          {activeTab === 'wrong-answers' ? renderWrongAnswers() : renderPracticeSheets()}
        </div>
      </div>

      <ConfirmDialog
        isOpen={!!deleteSheetId}
        title="删除练习卷"
        message="确定要删除这份练习卷吗？此操作不可恢复，已生成的文件也会被删除。"
        confirmText="删除"
        cancelText="取消"
        onConfirm={handleConfirmDelete}
        onCancel={handleCancelDelete}
        isDestructive={true}
      />
    </div>
  );
};
