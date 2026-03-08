/**
 * 学生详情页面组件
 * 展示单个学生的详细信息，包含标签、成绩、观察记录、家校沟通等 tab 页签
 */

import React, { useState } from 'react';
import { useParams, useNavigate } from 'react-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  commands,
  ParentCommunication,
  GenerateParentCommInput,
  UpdateParentCommunicationInput,
  RegenerateParentCommInput,
} from '@/bindings';
import { useToast } from '@/hooks/useToast';
import { EmptyState } from '@/components/shared/EmptyState';
import {
  ArrowLeft,
  Tag,
  FileText,
  MessageSquare,
  Award,
  Plus,
  X,
  Sparkles,
  RefreshCw,
  Check,
  Edit3,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';

export const StudentDetailPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { success, error } = useToast();
  const [activeTab, setActiveTab] = useState<'tags' | 'scores' | 'observations' | 'communications'>(
    'tags',
  );
  const [newTag, setNewTag] = useState('');

  // 标签内联编辑状态
  const [editingTagId, setEditingTagId] = useState<string | null>(null);
  const [editingTagName, setEditingTagName] = useState('');

  // AI 生成相关状态
  const [isAiGenerating, setIsAiGenerating] = useState(false);
  const [aiKeyword, setAiKeyword] = useState('');
  const [aiTone, setAiTone] = useState<string>('balanced'); // 'positive' | 'balanced' | 'constructive'
  const [editingDraftId, setEditingDraftId] = useState<string | null>(null);
  const [editingDraftText, setEditingDraftText] = useState('');
  const [showAiPanel, setShowAiPanel] = useState(false);

  // 弹窗状态
  const [isScoreModalOpen, setIsScoreModalOpen] = useState(false);
  const [isObservationModalOpen, setIsObservationModalOpen] = useState(false);
  const [isCommunicationModalOpen, setIsCommunicationModalOpen] = useState(false);
  // 表单状态
  const [scoreForm, setScoreForm] = useState({
    exam_name: '',
    subject: '',
    score: '',
    full_score: '100',
    rank_in_class: '',
    exam_date: new Date().toISOString().split('T')[0],
  });

  const [observationForm, setObservationForm] = useState({
    content: '',
    source: '',
  });

  const [communicationForm, setCommunicationForm] = useState({
    draft: '',
    status: 'draft' as 'draft' | 'adopted' | 'rejected',
  });
  const { data: student, isLoading: isStudentLoading } = useQuery({
    queryKey: ['student', id],
    queryFn: async () => {
      if (!id) throw new Error('No ID');
      const result = await commands.getStudent(id);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!id,
  });

  const { data: tags } = useQuery({
    queryKey: ['studentTags', id],
    queryFn: async () => {
      if (!id) throw new Error('No ID');
      const result = await commands.listStudentTags({ student_id: id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!id && activeTab === 'tags',
  });
  const { data: scores } = useQuery({
    queryKey: ['studentScores', id],
    queryFn: async () => {
      if (!id) throw new Error('No ID');
      const result = await commands.listStudentScores({
        student_id: id,
        subject: null,
        from_date: null,
        to_date: null,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!id && activeTab === 'scores',
  });

  const { data: observations } = useQuery({
    queryKey: ['studentObservations', id],
    queryFn: async () => {
      if (!id) throw new Error('No ID');
      const result = await commands.listStudentObservations({
        student_id: id,
        limit: null,
        offset: null,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!id && activeTab === 'observations',
  });

  const { data: communications } = useQuery({
    queryKey: ['parentCommunications', id],
    queryFn: async () => {
      if (!id) throw new Error('No ID');
      const result = await commands.listParentCommunications({ student_id: id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!id && activeTab === 'communications',
  });

  const addTagMutation = useMutation({
    mutationFn: async (tagName: string) => {
      if (!id) throw new Error('No ID');
      const result = await commands.addStudentTag({ student_id: id, tag_name: tagName });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['studentTags', id] });
      setNewTag('');
      success('标签添加成功');
    },
    onError: (err) => error(`添加失败: ${err.message}`),
  });

  const removeTagMutation = useMutation({
    mutationFn: async (tagId: string) => {
      const result = await commands.removeStudentTag({ id: tagId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['studentTags', id] });
      success('标签移除成功');
    },
    onError: (err) => error(`移除失败: ${err.message}`),
  });

  const updateTagMutation = useMutation({
    mutationFn: async ({ tagId, tag_name }: { tagId: string; tag_name: string }) => {
      const result = await commands.updateStudentTag({ id: tagId, tag_name });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['studentTags', id] });
      setEditingTagId(null);
      success('标签更新成功');
    },
    onError: (err) => error(`更新失败: ${err.message}`),
  });

  const createScoreMutation = useMutation({
    mutationFn: async (data: Omit<import('@/bindings').CreateScoreRecordInput, 'student_id'>) => {
      if (!id) throw new Error('No ID');
      const result = await commands.createScoreRecord({ student_id: id, ...data });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['studentScores', id] });
      setIsScoreModalOpen(false);
      setScoreForm({
        exam_name: '',
        subject: '',
        score: '',
        full_score: '100',
        rank_in_class: '',
        exam_date: new Date().toISOString().split('T')[0],
      });
      success('成绩添加成功');
    },
    onError: (err) => error(`添加失败: ${err.message}`),
  });

  const createObservationMutation = useMutation({
    mutationFn: async (
      data: Omit<import('@/bindings').CreateObservationNoteInput, 'student_id'>,
    ) => {
      if (!id) throw new Error('No ID');
      const result = await commands.createObservationNote({ student_id: id, ...data });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['studentObservations', id] });
      setIsObservationModalOpen(false);
      setObservationForm({ content: '', source: '' });
      success('记录添加成功');
    },
    onError: (err) => error(`添加失败: ${err.message}`),
  });

  const createCommunicationMutation = useMutation({
    mutationFn: async (
      data: Omit<import('@/bindings').CreateParentCommunicationInput, 'student_id'>,
    ) => {
      if (!id) throw new Error('No ID');
      const result = await commands.createParentCommunication({ student_id: id, ...data });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['parentCommunications', id] });
      setIsCommunicationModalOpen(false);
      setCommunicationForm({ draft: '', status: 'draft' });
      success('沟通添加成功');
    },
    onError: (err) => error(`添加失败: ${err.message}`),
  });

  const generateCommMutation = useMutation({
    mutationFn: async (input: GenerateParentCommInput) => {
      const result = await commands.generateParentCommunication(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['parentCommunications', id] });
      success('AI 文案生成成功');
      setIsAiGenerating(false);
    },
    onError: (err) => {
      error(`生成失败: ${err.message}`);
      setIsAiGenerating(false);
    },
  });

  const regenerateCommMutation = useMutation({
    mutationFn: async (input: RegenerateParentCommInput) => {
      const result = await commands.regenerateParentCommunication(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['parentCommunications', id] });
      success('重新生成成功');
    },
    onError: (err) => error(`重新生成失败: ${err.message}`),
  });

  const updateCommMutation = useMutation({
    mutationFn: async (input: UpdateParentCommunicationInput) => {
      const result = await commands.updateParentCommunication(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['parentCommunications', id] });
      success('更新成功');
      setEditingDraftId(null);
    },
    onError: (err) => error(`更新失败: ${err.message}`),
  });
  const handleAddTag = (e: React.FormEvent) => {
    e.preventDefault();
    if (newTag.trim()) {
      addTagMutation.mutate(newTag.trim());
    }
  };

  /**
   * 处理标签更新
   */
  const handleUpdateTag = (tagId: string) => {
    if (editingTagName.trim()) {
      updateTagMutation.mutate({ tagId, tag_name: editingTagName.trim() });
    } else {
      setEditingTagId(null);
    }
  };

  /**
   * 处理成绩创建
   */
  const handleCreateScore = (e: React.FormEvent) => {
    e.preventDefault();
    createScoreMutation.mutate({
      exam_name: scoreForm.exam_name,
      subject: scoreForm.subject,
      score: Number(scoreForm.score),
      full_score: Number(scoreForm.full_score),
      rank_in_class: scoreForm.rank_in_class ? Number(scoreForm.rank_in_class) : null,
      exam_date: scoreForm.exam_date,
    });
  };

  /**
   * 处理观察记录创建
   */
  const handleCreateObservation = (e: React.FormEvent) => {
    e.preventDefault();
    createObservationMutation.mutate({
      content: observationForm.content,
      source: observationForm.source || null,
    });
  };

  /**
   * 处理家校沟通创建
   */
  const handleCreateCommunication = (e: React.FormEvent) => {
    e.preventDefault();
    createCommunicationMutation.mutate({
      draft: communicationForm.draft,
      status: communicationForm.status,
      adopted_text: null,
      evidence_json: null,
    });
  };

  const handleAiGenerate = () => {
    setIsAiGenerating(true);
    generateCommMutation.mutate({
      student_id: id!,
      keyword: aiKeyword || null,
      tone: aiTone || null,
    });
  };

  const handleAdopt = (comm: ParentCommunication) => {
    updateCommMutation.mutate({
      id: comm.id,
      draft: null,
      adopted_text: comm.draft,
      status: 'adopted',
      evidence_json: null,
    });
  };

  const handleReject = (commId: string) => {
    updateCommMutation.mutate({
      id: commId,
      draft: null,
      adopted_text: null,
      status: 'rejected',
      evidence_json: null,
    });
  };

  const handleSaveEdit = (commId: string) => {
    updateCommMutation.mutate({
      id: commId,
      draft: editingDraftText,
      adopted_text: null,
      status: null,
      evidence_json: null,
    });
  };

  const parseDraftSections = (
    draft: string,
  ): { affirmation: string; issue: string; suggestion: string } | null => {
    const affMatch = draft.match(/【肯定】([\s\S]*?)(?=【问题】|$)/);
    const issMatch = draft.match(/【问题】([\s\S]*?)(?=【建议】|$)/);
    const sugMatch = draft.match(/【建议】([\s\S]*?)$/);
    if (affMatch && issMatch && sugMatch) {
      return {
        affirmation: affMatch[1].trim(),
        issue: issMatch[1].trim(),
        suggestion: sugMatch[1].trim(),
      };
    }
    return null;
  };
  if (isStudentLoading) {
    return <div className="p-8 text-center text-gray-500">加载中...</div>;
  }

  if (!student) {
    return <div className="p-8 text-center text-red-500">未找到学生信息</div>;
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center gap-4">
        <button
          onClick={() => navigate('/students')}
          className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          aria-label="返回"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <div>
          <h1 className="text-2xl font-bold text-gray-900 flex items-center gap-3">
            {student.name}
            <span
              className={`text-sm px-2 py-0.5 rounded font-medium ${
                student.gender === '男'
                  ? 'bg-blue-50 text-blue-700'
                  : student.gender === '女'
                    ? 'bg-pink-50 text-pink-700'
                    : 'bg-gray-100 text-gray-700'
              }`}
            >
              {student.gender || '未知'}
            </span>
          </h1>
          <p className="text-sm text-gray-500 mt-1 font-mono">学号: {student.student_no}</p>
        </div>
      </header>

      <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div className="flex border-b border-gray-100">
          <button
            onClick={() => setActiveTab('tags')}
            className={`flex items-center gap-2 px-6 py-4 text-sm font-medium transition-colors relative ${
              activeTab === 'tags'
                ? 'text-brand-600'
                : 'text-gray-500 hover:text-gray-900 hover:bg-gray-50'
            }`}
          >
            <Tag className="w-4 h-4" />
            标签
            {activeTab === 'tags' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-brand-600" />
            )}
          </button>
          <button
            onClick={() => setActiveTab('scores')}
            className={`flex items-center gap-2 px-6 py-4 text-sm font-medium transition-colors relative ${
              activeTab === 'scores'
                ? 'text-brand-600'
                : 'text-gray-500 hover:text-gray-900 hover:bg-gray-50'
            }`}
          >
            <Award className="w-4 h-4" />
            成绩
            {activeTab === 'scores' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-brand-600" />
            )}
          </button>
          <button
            onClick={() => setActiveTab('observations')}
            className={`flex items-center gap-2 px-6 py-4 text-sm font-medium transition-colors relative ${
              activeTab === 'observations'
                ? 'text-brand-600'
                : 'text-gray-500 hover:text-gray-900 hover:bg-gray-50'
            }`}
          >
            <FileText className="w-4 h-4" />
            观察记录
            {activeTab === 'observations' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-brand-600" />
            )}
          </button>
          <button
            onClick={() => setActiveTab('communications')}
            className={`flex items-center gap-2 px-6 py-4 text-sm font-medium transition-colors relative ${
              activeTab === 'communications'
                ? 'text-brand-600'
                : 'text-gray-500 hover:text-gray-900 hover:bg-gray-50'
            }`}
          >
            <MessageSquare className="w-4 h-4" />
            家校沟通
            {activeTab === 'communications' && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-brand-600" />
            )}
          </button>
        </div>

        <div className="p-6 min-h-[400px]">
          {activeTab === 'tags' && (
            <div className="space-y-6">
              <form onSubmit={handleAddTag} className="flex gap-3 max-w-md">
                <input
                  type="text"
                  value={newTag}
                  onChange={(e) => setNewTag(e.target.value)}
                  placeholder="输入新标签..."
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                />
                <button
                  type="submit"
                  disabled={!newTag.trim() || addTagMutation.isPending}
                  className="flex items-center gap-2 px-4 py-2 bg-brand-50 text-brand-700 rounded-lg hover:bg-brand-100 transition-colors font-medium text-sm disabled:opacity-50"
                >
                  <Plus className="w-4 h-4" />
                  添加
                </button>
              </form>

              <div className="flex flex-wrap gap-2">
                {tags?.map((tag) => (
                  <span
                    key={tag.id}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium bg-gray-100 text-gray-700 group"
                    onDoubleClick={() => {
                      setEditingTagId(tag.id);
                      setEditingTagName(tag.tag_name);
                    }}
                  >
                    {editingTagId === tag.id ? (
                      <input
                        type="text"
                        value={editingTagName}
                        onChange={(e) => setEditingTagName(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleUpdateTag(tag.id);
                          if (e.key === 'Escape') setEditingTagId(null);
                        }}
                        onBlur={() => setEditingTagId(null)}
                        autoFocus
                        className="bg-transparent border-none outline-none w-20 text-sm p-0 m-0"
                      />
                    ) : (
                      tag.tag_name
                    )}
                    <button
                      onClick={() => removeTagMutation.mutate(tag.id)}
                      className="p-0.5 rounded-full text-gray-400 hover:text-red-600 hover:bg-red-50 transition-colors opacity-0 group-hover:opacity-100"
                      aria-label="移除标签"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </span>
                ))}
                {(!tags || tags.length === 0) && (
                  <p className="text-sm text-gray-500 italic">暂无标签</p>
                )}
              </div>
            </div>
          )}

          {activeTab === 'scores' && (
            <div className="space-y-4">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium text-gray-900">成绩记录</h3>
                <button
                  onClick={() => setIsScoreModalOpen(true)}
                  className="flex items-center gap-2 px-3 py-1.5 bg-brand-50 text-brand-700 rounded-lg hover:bg-brand-100 transition-colors text-sm font-medium"
                >
                  <Plus className="w-4 h-4" />
                  添加成绩
                </button>
              </div>
              {scores && scores.length > 0 ? (
                <div className="overflow-x-auto border border-gray-100 rounded-lg">
                  <table className="w-full text-left border-collapse">
                    <thead>
                      <tr className="bg-gray-50 border-b border-gray-100 text-sm font-medium text-gray-500">
                        <th className="px-4 py-3">考试名称</th>
                        <th className="px-4 py-3">科目</th>
                        <th className="px-4 py-3">成绩</th>
                        <th className="px-4 py-3">满分</th>
                        <th className="px-4 py-3">班级排名</th>
                        <th className="px-4 py-3">考试日期</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-100">
                      {scores.map((score) => (
                        <tr
                          key={score.id}
                          className="hover:bg-gray-50/50 transition-colors text-sm"
                        >
                          <td className="px-4 py-3 font-medium text-gray-900">{score.exam_name}</td>
                          <td className="px-4 py-3 text-gray-600">{score.subject}</td>
                          <td className="px-4 py-3 font-semibold text-brand-600">{score.score}</td>
                          <td className="px-4 py-3 text-gray-500">{score.full_score}</td>
                          <td className="px-4 py-3 text-gray-600">{score.rank_in_class || '-'}</td>
                          <td className="px-4 py-3 text-gray-500">{score.exam_date}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <EmptyState
                  icon={<Award className="w-8 h-8" />}
                  title="暂无成绩记录"
                  description="该学生目前没有成绩记录。"
                />
              )}
            </div>
          )}

          {activeTab === 'observations' && (
            <div className="space-y-4">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium text-gray-900">观察记录</h3>
                <button
                  onClick={() => setIsObservationModalOpen(true)}
                  className="flex items-center gap-2 px-3 py-1.5 bg-brand-50 text-brand-700 rounded-lg hover:bg-brand-100 transition-colors text-sm font-medium"
                >
                  <Plus className="w-4 h-4" />
                  添加记录
                </button>
              </div>
              {observations && observations.length > 0 ? (
                <div className="space-y-3">
                  {observations.map((obs) => (
                    <div
                      key={obs.id}
                      className="p-4 border border-gray-100 rounded-lg bg-gray-50/50"
                    >
                      <p className="text-gray-800 text-sm whitespace-pre-wrap">{obs.content}</p>
                      <div className="mt-2 flex items-center justify-between text-xs text-gray-500">
                        <span>来源: {obs.source || '手动记录'}</span>
                        <span>{new Date(obs.created_at).toLocaleString('zh-CN')}</span>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <EmptyState
                  icon={<FileText className="w-8 h-8" />}
                  title="暂无观察记录"
                  description="该学生目前没有观察记录。"
                />
              )}
            </div>
          )}

          {activeTab === 'communications' && (
            <div className="space-y-4">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium text-gray-900">家校沟通</h3>
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowAiPanel(!showAiPanel)}
                    className="flex items-center gap-2 px-3 py-1.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors text-sm font-medium"
                  >
                    <Sparkles className="w-4 h-4" />
                    AI 生成文案
                  </button>
                  <button
                    onClick={() => setIsCommunicationModalOpen(true)}
                    className="flex items-center gap-2 px-3 py-1.5 bg-brand-50 text-brand-700 rounded-lg hover:bg-brand-100 transition-colors text-sm font-medium"
                  >
                    <Plus className="w-4 h-4" />
                    新建沟通
                  </button>
                </div>
              </div>

              {showAiPanel && (
                <div className="p-4 bg-brand-50 rounded-xl border border-brand-100 space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      关键词 (可选)
                    </label>
                    <input
                      type="text"
                      value={aiKeyword}
                      onChange={(e) => setAiKeyword(e.target.value)}
                      placeholder="如：学习进步、课堂表现"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm bg-white"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">语气基调</label>
                    <div className="flex gap-4">
                      <label className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
                        <input
                          type="radio"
                          name="tone"
                          value="positive"
                          checked={aiTone === 'positive'}
                          onChange={(e) => setAiTone(e.target.value)}
                          className="text-brand-600 focus:ring-brand-500"
                        />
                        正面鼓励
                      </label>
                      <label className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
                        <input
                          type="radio"
                          name="tone"
                          value="balanced"
                          checked={aiTone === 'balanced'}
                          onChange={(e) => setAiTone(e.target.value)}
                          className="text-brand-600 focus:ring-brand-500"
                        />
                        均衡客观
                      </label>
                      <label className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
                        <input
                          type="radio"
                          name="tone"
                          value="constructive"
                          checked={aiTone === 'constructive'}
                          onChange={(e) => setAiTone(e.target.value)}
                          className="text-brand-600 focus:ring-brand-500"
                        />
                        建设性意见
                      </label>
                    </div>
                  </div>
                  <div className="flex justify-end">
                    <button
                      onClick={handleAiGenerate}
                      disabled={isAiGenerating}
                      className="flex items-center gap-2 px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors text-sm font-medium disabled:opacity-50"
                    >
                      {isAiGenerating ? (
                        <>
                          <RefreshCw className="w-4 h-4 animate-spin" />
                          AI 正在生成文案...
                        </>
                      ) : (
                        <>
                          <Sparkles className="w-4 h-4" />
                          开始生成
                        </>
                      )}
                    </button>
                  </div>
                </div>
              )}

              {communications && communications.length > 0 ? (
                <div className="space-y-4">
                  {communications.map((comm) => {
                    const isDraft = comm.status === 'draft';
                    const draftText = comm.adopted_text || comm.draft || '';
                    const parsedSections = parseDraftSections(draftText);
                    const isEditing = editingDraftId === comm.id;

                    let evidenceList: Record<string, unknown>[] = [];
                    if (comm.evidence_json) {
                      try {
                        evidenceList = JSON.parse(comm.evidence_json);
                      } catch {
                        // ignore
                      }
                    }

                    return (
                      <div
                        key={comm.id}
                        className="p-4 border border-gray-100 rounded-xl bg-white shadow-sm relative overflow-hidden"
                      >
                        {/* AI Badge if it has evidence or parsed sections */}
                        {(parsedSections || comm.evidence_json) && (
                          <div className="absolute top-0 right-0 bg-brand-50 text-brand-600 text-xs px-2 py-1 rounded-bl-lg flex items-center gap-1 font-medium">
                            <Sparkles className="w-3 h-3" />
                            AI 生成
                          </div>
                        )}

                        <div className="flex items-center justify-between mb-3">
                          <span
                            className={`px-2 py-0.5 rounded text-xs font-medium ${
                              comm.status === 'draft'
                                ? 'bg-yellow-50 text-yellow-700'
                                : comm.status === 'adopted'
                                  ? 'bg-green-50 text-green-700'
                                  : comm.status === 'rejected'
                                    ? 'bg-red-50 text-red-700'
                                    : 'bg-gray-100 text-gray-700'
                            }`}
                          >
                            {comm.status === 'draft'
                              ? '草稿'
                              : comm.status === 'adopted'
                                ? '已采纳'
                                : comm.status === 'rejected'
                                  ? '已拒绝'
                                  : comm.status || '未知状态'}
                          </span>
                          <span className="text-xs text-gray-500">
                            {new Date(comm.created_at).toLocaleString('zh-CN')}
                          </span>
                        </div>

                        {isEditing ? (
                          <div className="space-y-3">
                            <textarea
                              rows={6}
                              value={editingDraftText}
                              onChange={(e) => setEditingDraftText(e.target.value)}
                              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm resize-none"
                            />
                            <div className="flex justify-end gap-2">
                              <button
                                onClick={() => setEditingDraftId(null)}
                                className="px-3 py-1.5 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors text-sm font-medium"
                              >
                                取消
                              </button>
                              <button
                                onClick={() => handleSaveEdit(comm.id)}
                                disabled={updateCommMutation.isPending}
                                className="px-3 py-1.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors text-sm font-medium disabled:opacity-50"
                              >
                                保存
                              </button>
                            </div>
                          </div>
                        ) : (
                          <>
                            {parsedSections ? (
                              <div className="space-y-3 mb-4">
                                <div className="bg-green-50 border-l-4 border-green-400 p-3 rounded-r-lg">
                                  <h4 className="text-green-800 text-sm font-medium mb-1 flex items-center gap-1">
                                    <Check className="w-4 h-4" /> 肯定
                                  </h4>
                                  <p className="text-gray-800 text-sm whitespace-pre-wrap">
                                    {parsedSections.affirmation}
                                  </p>
                                </div>
                                <div className="bg-amber-50 border-l-4 border-amber-400 p-3 rounded-r-lg">
                                  <h4 className="text-amber-800 text-sm font-medium mb-1 flex items-center gap-1">
                                    <Edit3 className="w-4 h-4" /> 问题
                                  </h4>
                                  <p className="text-gray-800 text-sm whitespace-pre-wrap">
                                    {parsedSections.issue}
                                  </p>
                                </div>
                                <div className="bg-blue-50 border-l-4 border-blue-400 p-3 rounded-r-lg">
                                  <h4 className="text-blue-800 text-sm font-medium mb-1 flex items-center gap-1">
                                    <Sparkles className="w-4 h-4" /> 建议
                                  </h4>
                                  <p className="text-gray-800 text-sm whitespace-pre-wrap">
                                    {parsedSections.suggestion}
                                  </p>
                                </div>
                              </div>
                            ) : (
                              <p className="text-gray-800 text-sm whitespace-pre-wrap mb-4">
                                {draftText || '无内容'}
                              </p>
                            )}

                            {evidenceList.length > 0 && (
                              <details className="group mb-4">
                                <summary className="flex items-center gap-1 text-xs text-gray-500 cursor-pointer hover:text-gray-700 select-none">
                                  <ChevronDown className="w-3 h-3 group-open:hidden" />
                                  <ChevronUp className="w-3 h-3 hidden group-open:block" />
                                  参考依据 ({evidenceList.length})
                                </summary>
                                <div className="mt-2 space-y-2 pl-4 border-l-2 border-gray-100">
                                  {evidenceList.map((ev, idx) => (
                                    <div
                                      key={idx}
                                      className="text-xs text-gray-600 bg-gray-50 p-2 rounded"
                                    >
                                      <div className="font-medium text-gray-700 mb-1">
                                        {String(ev.source || '记录')}{' '}
                                        {ev.date ? `(${String(ev.date)})` : ''}
                                      </div>
                                      <div>{String(ev.content || '')}</div>
                                    </div>
                                  ))}
                                </div>
                              </details>
                            )}

                            {isDraft && (
                              <div className="flex items-center gap-2 pt-3 border-t border-gray-100">
                                <button
                                  onClick={() => {
                                    setEditingDraftId(comm.id);
                                    setEditingDraftText(comm.draft || '');
                                  }}
                                  className="flex items-center gap-1.5 px-3 py-1.5 text-gray-600 hover:text-brand-600 hover:bg-brand-50 rounded-lg transition-colors text-sm font-medium"
                                >
                                  <Edit3 className="w-4 h-4" />
                                  编辑
                                </button>
                                <button
                                  onClick={() => handleAdopt(comm)}
                                  disabled={updateCommMutation.isPending}
                                  className="flex items-center gap-1.5 px-3 py-1.5 text-green-600 hover:text-green-700 hover:bg-green-50 rounded-lg transition-colors text-sm font-medium"
                                >
                                  <Check className="w-4 h-4" />
                                  采纳
                                </button>
                                <button
                                  onClick={() => handleReject(comm.id)}
                                  disabled={updateCommMutation.isPending}
                                  className="flex items-center gap-1.5 px-3 py-1.5 text-red-600 hover:text-red-700 hover:bg-red-50 rounded-lg transition-colors text-sm font-medium"
                                >
                                  <X className="w-4 h-4" />
                                  拒绝
                                </button>
                                <button
                                  onClick={() =>
                                    regenerateCommMutation.mutate({
                                      id: comm.id,
                                      tone: aiTone || null,
                                    })
                                  }
                                  disabled={regenerateCommMutation.isPending}
                                  className="flex items-center gap-1.5 px-3 py-1.5 text-gray-600 hover:text-brand-600 hover:bg-brand-50 rounded-lg transition-colors text-sm font-medium ml-auto"
                                >
                                  <RefreshCw
                                    className={`w-4 h-4 ${regenerateCommMutation.isPending ? 'animate-spin' : ''}`}
                                  />
                                  重新生成
                                </button>
                              </div>
                            )}
                          </>
                        )}
                      </div>
                    );
                  })}
                </div>
              ) : (
                <EmptyState
                  icon={<MessageSquare className="w-8 h-8" />}
                  title="暂无沟通记录"
                  description="该学生目前没有家校沟通记录。"
                />
              )}
            </div>
          )}
        </div>
      </div>

      {/* 成绩表单弹窗 */}
      {isScoreModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-lg w-full max-w-md overflow-hidden">
            <div className="flex justify-between items-center p-4 border-b border-gray-100">
              <h3 className="text-lg font-medium text-gray-900">添加成绩</h3>
              <button
                onClick={() => setIsScoreModalOpen(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <form onSubmit={handleCreateScore} className="p-4 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">考试名称</label>
                <input
                  required
                  type="text"
                  value={scoreForm.exam_name}
                  onChange={(e) => setScoreForm({ ...scoreForm, exam_name: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  placeholder="如：期中考试"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">科目</label>
                <input
                  required
                  type="text"
                  value={scoreForm.subject}
                  onChange={(e) => setScoreForm({ ...scoreForm, subject: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  placeholder="如：语文"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">成绩</label>
                  <input
                    required
                    type="number"
                    step="0.1"
                    value={scoreForm.score}
                    onChange={(e) => setScoreForm({ ...scoreForm, score: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">满分</label>
                  <input
                    required
                    type="number"
                    step="0.1"
                    value={scoreForm.full_score}
                    onChange={(e) => setScoreForm({ ...scoreForm, full_score: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  />
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    班级排名 (可选)
                  </label>
                  <input
                    type="number"
                    value={scoreForm.rank_in_class}
                    onChange={(e) => setScoreForm({ ...scoreForm, rank_in_class: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">考试日期</label>
                  <input
                    required
                    type="date"
                    value={scoreForm.exam_date}
                    onChange={(e) => setScoreForm({ ...scoreForm, exam_date: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  />
                </div>
              </div>
              <div className="flex justify-end gap-3 pt-4 border-t border-gray-100">
                <button
                  type="button"
                  onClick={() => setIsScoreModalOpen(false)}
                  className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors text-sm font-medium"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={createScoreMutation.isPending}
                  className="px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors text-sm font-medium disabled:opacity-50"
                >
                  保存
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 观察记录表单弹窗 */}
      {isObservationModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-lg w-full max-w-md overflow-hidden">
            <div className="flex justify-between items-center p-4 border-b border-gray-100">
              <h3 className="text-lg font-medium text-gray-900">添加观察记录</h3>
              <button
                onClick={() => setIsObservationModalOpen(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <form onSubmit={handleCreateObservation} className="p-4 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">记录内容</label>
                <textarea
                  required
                  rows={4}
                  value={observationForm.content}
                  onChange={(e) =>
                    setObservationForm({ ...observationForm, content: e.target.value })
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm resize-none"
                  placeholder="输入观察记录内容..."
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">来源 (可选)</label>
                <input
                  type="text"
                  value={observationForm.source}
                  onChange={(e) =>
                    setObservationForm({ ...observationForm, source: e.target.value })
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm"
                  placeholder="如：课堂观察、家访"
                />
              </div>
              <div className="flex justify-end gap-3 pt-4 border-t border-gray-100">
                <button
                  type="button"
                  onClick={() => setIsObservationModalOpen(false)}
                  className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors text-sm font-medium"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={createObservationMutation.isPending}
                  className="px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors text-sm font-medium disabled:opacity-50"
                >
                  保存
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 家校沟通表单弹窗 */}
      {isCommunicationModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-lg w-full max-w-md overflow-hidden">
            <div className="flex justify-between items-center p-4 border-b border-gray-100">
              <h3 className="text-lg font-medium text-gray-900">新建家校沟通</h3>
              <button
                onClick={() => setIsCommunicationModalOpen(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <form onSubmit={handleCreateCommunication} className="p-4 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  沟通内容 (草稿)
                </label>
                <textarea
                  required
                  rows={4}
                  value={communicationForm.draft}
                  onChange={(e) =>
                    setCommunicationForm({ ...communicationForm, draft: e.target.value })
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm resize-none"
                  placeholder="输入沟通内容..."
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">状态</label>
                <select
                  value={communicationForm.status}
                  onChange={(e) =>
                    setCommunicationForm({
                      ...communicationForm,
                      status: e.target.value as 'draft' | 'adopted' | 'rejected',
                    })
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow text-sm bg-white"
                >
                  <option value="draft">草稿</option>
                  <option value="adopted">已采纳</option>
                  <option value="rejected">已拒绝</option>
                </select>
              </div>
              <div className="flex justify-end gap-3 pt-4 border-t border-gray-100">
                <button
                  type="button"
                  onClick={() => setIsCommunicationModalOpen(false)}
                  className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors text-sm font-medium"
                >
                  取消
                </button>
                <button
                  type="submit"
                  disabled={createCommunicationMutation.isPending}
                  className="px-4 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors text-sm font-medium disabled:opacity-50"
                >
                  保存
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
