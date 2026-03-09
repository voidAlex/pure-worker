/**
 * 作业批改页面组件
 * 支持创建批改任务、拖拽上传作业图片、启动OCR批改、进度监控、结果复核与冲突解决、导出Excel
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands,
type Classroom,
type GradingJob,
type AssignmentAsset,
type AssignmentOcrResult,
type AsyncTask,
type Student,
type CreateGradingJobInput,
type AddAssignmentAssetsInput,
type ReviewOcrResultInput, } from '@/services/commandClient';
import { useToast } from '@/hooks/useToast';
import { EmptyState } from '@/components/shared/EmptyState';
import { ConfirmDialog } from '@/components/shared/ConfirmDialog';
import { isTauriRuntime } from '@/utils/runtime';
import {
  ClipboardCheck,
  Upload,
  Play,
  Trash2,
  AlertTriangle,
  Check,
  X,
  FileSpreadsheet,
  Plus,
} from 'lucide-react';
/** 任务状态徽章颜色映射 */
const JOB_STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  pending: { bg: 'bg-gray-100', text: 'text-gray-600', label: '待处理' },
  processing: { bg: 'bg-blue-100', text: 'text-blue-600', label: '处理中' },
  completed: { bg: 'bg-green-100', text: 'text-green-700', label: '已完成' },
  failed: { bg: 'bg-red-100', text: 'text-red-600', label: '失败' },
};

/** 审核状态徽章颜色映射 */
const REVIEW_STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  pending: { bg: 'bg-gray-100', text: 'text-gray-600', label: '待审核' },
  approved: { bg: 'bg-green-100', text: 'text-green-700', label: '已确认' },
  rejected: { bg: 'bg-red-100', text: 'text-red-600', label: '已拒绝' },
};

type GradingProgress = {
  total: number;
  processed: number;
  failed: number;
  conflicts: number;
  current_filename: string | null;
};

export const AssignmentGradingPage: React.FC = () => {
  const queryClient = useQueryClient();
  const { success, error } = useToast();

  // 状态管理
  const [selectedClassId, setSelectedClassId] = useState<string | null>(null);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const [isGrading, setIsGrading] = useState(false);
  const [gradingProgress, setGradingProgress] = useState<GradingProgress | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);
  const [deleteTargetType, setDeleteTargetType] = useState<'job' | 'asset' | null>(null);
  
  // 创建任务表单状态
  const [newJobTitle, setNewJobTitle] = useState('');
  const [newJobMode, setNewJobMode] = useState('standard');
  const [showCreateForm, setShowCreateForm] = useState(false);

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
      const result = await commands.listStudents({ class_id: selectedClassId! });
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

  /** 获取学生姓名 */
  const getStudentName = useCallback(
    (studentId: string | null): string => {
      if (!studentId) return '未匹配';
      return studentMap.get(studentId)?.name || studentId;
    },
    [studentMap],
  );

  /** 查询批改任务列表 */
  const { data: gradingJobs, isLoading: isLoadingJobs } = useQuery({
    queryKey: ['gradingJobs', selectedClassId],
    queryFn: async () => {
      const result = await commands.listGradingJobs({ class_id: selectedClassId! });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedClassId,
  });

  /** 当前选中的任务 */
  const selectedJob = useMemo(() => {
    return gradingJobs?.find((j) => j.id === selectedJobId) || null;
  }, [gradingJobs, selectedJobId]);

  /** 查询任务素材列表 */
  const { data: jobAssets } = useQuery({
    queryKey: ['jobAssets', selectedJobId],
    queryFn: async () => {
      const result = await commands.listJobAssets({ job_id: selectedJobId! });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedJobId,
  });

  /** 查询 OCR 结果列表 */
  const { data: ocrResults } = useQuery({
    queryKey: ['ocrResults', selectedJobId],
    queryFn: async () => {
      const result = await commands.listJobOcrResults({ job_id: selectedJobId! });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedJobId,
  });

  /** 查询冲突结果列表 */
  const { data: conflictResults } = useQuery({
    queryKey: ['conflictResults', selectedJobId],
    queryFn: async () => {
      const result = await commands.listConflictResults({ job_id: selectedJobId! });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    enabled: !!selectedJobId,
  });

  /** 创建批改任务 */
  const createJobMutation = useMutation({
    mutationFn: async (input: CreateGradingJobInput) => {
      const result = await commands.createGradingJob(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['gradingJobs', selectedClassId] });
      setSelectedJobId(data.id);
      setShowCreateForm(false);
      setNewJobTitle('');
      success('批改任务创建成功');
    },
    onError: (err) => error(`创建失败: ${err.message}`),
  });

  /** 添加作业素材 */
  const addAssetsMutation = useMutation({
    mutationFn: async (input: AddAssignmentAssetsInput) => {
      const result = await commands.addAssignmentAssets(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['jobAssets', selectedJobId] });
      queryClient.invalidateQueries({ queryKey: ['gradingJobs', selectedClassId] });
      success('作业图片添加成功');
    },
    onError: (err) => error(`添加图片失败: ${err.message}`),
  });

  /** 启动批改任务 */
  const startGradingMutation = useMutation({
    mutationFn: async (jobId: string) => {
      const result = await commands.startGrading({ job_id: jobId });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      setIsGrading(true);
      setGradingProgress(null);
      queryClient.invalidateQueries({ queryKey: ['gradingJobs', selectedClassId] });
      success('批改任务已启动');
    },
    onError: (err) => error(`启动批改失败: ${err.message}`),
  });

  /** 审核单条 OCR 结果 */
  const reviewMutation = useMutation({
    mutationFn: async (input: ReviewOcrResultInput) => {
      const result = await commands.reviewOcrResult(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ocrResults', selectedJobId] });
      queryClient.invalidateQueries({ queryKey: ['conflictResults', selectedJobId] });
      success('审核状态已更新');
    },
    onError: (err) => error(`审核失败: ${err.message}`),
  });

  /** 批量审核 OCR 结果 */
  const batchReviewMutation = useMutation({
    mutationFn: async (input: { ids: string[]; review_status: string; reviewed_by: string; final_score: number | null }) => {
      const result = await commands.batchReviewOcrResults(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ocrResults', selectedJobId] });
      queryClient.invalidateQueries({ queryKey: ['conflictResults', selectedJobId] });
      success('批量审核成功');
    },
    onError: (err) => error(`批量审核失败: ${err.message}`),
  });

  /** 删除任务 */
  const deleteJobMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.deleteGradingJob({ id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['gradingJobs', selectedClassId] });
      setSelectedJobId(null);
      setShowDeleteConfirm(false);
      success('任务已删除');
    },
    onError: (err) => error(`删除失败: ${err.message}`),
  });

  /** 删除素材 */
  const deleteAssetMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await commands.deleteAssignmentAsset({ id });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['jobAssets', selectedJobId] });
      queryClient.invalidateQueries({ queryKey: ['gradingJobs', selectedClassId] });
      setShowDeleteConfirm(false);
      success('素材已删除');
    },
    onError: (err) => error(`删除失败: ${err.message}`),
  });

  /** 监听拖拽文件事件 */
  useEffect(() => {
    if (!isTauriRuntime()) return;
    if (!selectedJobId || !selectedClassId) return;
    const unlisten = import('@tauri-apps/api/event').then(({ listen }) =>
      listen<{ paths: string[] }>('tauri://drag-drop', (event) => {
        const paths = event.payload.paths;
        if (paths.length > 0) {
          addAssetsMutation.mutate({
            job_id: selectedJobId,
            class_id: selectedClassId,
            file_paths: paths,
          });
        }
      }),
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [selectedJobId, selectedClassId, addAssetsMutation]);

  /** 轮询批改任务进度 */
  useEffect(() => {
    if (!selectedJob?.task_id || !isGrading) return;

    const intervalId = setInterval(async () => {
      try {
        const result = await commands.getBatchTaskProgress(selectedJob.task_id!);
        if (result.status === 'error') {
          console.error('获取进度失败:', result.error);
          return;
        }
        const task: AsyncTask = result.data;

        if (task.progress_json) {
          try {
            const progress = JSON.parse(task.progress_json) as GradingProgress;
            setGradingProgress(progress);
          } catch (err) {
            console.error('解析进度 JSON 失败:', err);
          }
        }

        if (task.status === 'completed' || task.status === 'failed') {
          setIsGrading(false);
          queryClient.invalidateQueries({ queryKey: ['gradingJobs', selectedClassId] });
          queryClient.invalidateQueries({ queryKey: ['ocrResults', selectedJobId] });
          queryClient.invalidateQueries({ queryKey: ['conflictResults', selectedJobId] });
          if (task.status === 'completed') {
            success('批改任务完成');
          } else {
            error(`批改任务失败: ${task.context_data || '未知错误'}`);
          }
        }
      } catch (err) {
        console.error('轮询进度异常:', err);
      }
    }, 2000);

    return () => clearInterval(intervalId);
  }, [selectedJob?.task_id, isGrading, queryClient, selectedClassId, selectedJobId, success, error]);

  /** 处理创建任务 */
  const handleCreateJob = useCallback(() => {
    if (!selectedClassId || !newJobTitle.trim()) return;
    createJobMutation.mutate({
      class_id: selectedClassId,
      title: newJobTitle.trim(),
      grading_mode: newJobMode,
      answer_key_json: null,
      scoring_rules_json: null,
      task_id: null,
      output_path: null,
    });
  }, [selectedClassId, newJobTitle, newJobMode, createJobMutation]);

  /** 处理单条审核 */
  const handleReview = useCallback(
    (id: string, status: string, score: number | null) => {
      reviewMutation.mutate({
        id,
        review_status: status,
        final_score: score,
        reviewed_by: 'teacher',
      });
    },
    [reviewMutation],
  );

  /** 处理批量审核通过 */
  const handleBatchApprove = useCallback(() => {
    if (!ocrResults) return;
    const pendingIds = ocrResults.filter((r) => r.review_status === 'pending').map((r) => r.id);
    if (pendingIds.length === 0) return;
    batchReviewMutation.mutate({
      ids: pendingIds,
      review_status: 'approved',
      reviewed_by: 'teacher',
      final_score: null,
    });
  }, [ocrResults, batchReviewMutation]);

  /** 处理导出 Excel */
  const handleExport = useCallback(async () => {
    if (!selectedJobId) return;
    try {
      const fileName = `作业批改_${Date.now()}.xlsx`;
      const result = await commands.exportGradingResults({
        job_id: selectedJobId,
        output_path: fileName,
      });
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      success(`已导出 ${result.data.total_rows} 条记录到 ${result.data.output_path}`);
    } catch (err) {
      error(`导出失败: ${err instanceof Error ? err.message : '未知错误'}`);
    }
  }, [selectedJobId, success, error]);

  /** 计算进度百分比 */
  const progressPercent = useMemo(() => {
    if (!gradingProgress || gradingProgress.total === 0) return 0;
    return Math.round(((gradingProgress.processed + gradingProgress.failed) / gradingProgress.total) * 100);
  }, [gradingProgress]);

  return (
    <div className="space-y-6">
      {/* 页面标题 */}
      <header>
        <h1 className="text-2xl font-bold text-gray-900">作业批改</h1>
        <p className="text-sm text-gray-500 mt-1">拍照上传作业，AI识别批改，教师复核确认</p>
      </header>

      {/* 班级与任务选择区域 */}
      <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
        <div className="flex flex-wrap items-end gap-4">
          {/* 班级选择 */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">选择班级</label>
            <select
              value={selectedClassId || ''}
              onChange={(e) => {
                setSelectedClassId(e.target.value || null);
                setSelectedJobId(null);
              }}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow bg-white"
              disabled={isGrading}
            >
              <option value="">{isLoadingClasses ? '加载中...' : '请选择班级'}</option>
              {classrooms?.map((cls: Classroom) => (
                <option key={cls.id} value={cls.id}>
                  {cls.grade} {cls.class_name} - {cls.subject}
                </option>
              ))}
            </select>
          </div>

          {/* 任务选择 */}
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-gray-700 mb-1">选择批改任务</label>
            <select
              value={selectedJobId || ''}
              onChange={(e) => setSelectedJobId(e.target.value || null)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow bg-white"
              disabled={!selectedClassId || isGrading}
            >
              <option value="">{isLoadingJobs ? '加载中...' : '请选择任务'}</option>
              {gradingJobs?.map((job: GradingJob) => (
                <option key={job.id} value={job.id}>
                  {job.title} ({JOB_STATUS_STYLES[job.status]?.label || job.status})
                </option>
              ))}
            </select>
          </div>

          {/* 新建任务按钮 */}
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            disabled={!selectedClassId || isGrading}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Plus className="w-4 h-4" />
            新建任务
          </button>
        </div>

        {/* 新建任务表单 */}
        {showCreateForm && (
          <div className="mt-4 p-4 bg-gray-50 rounded-lg border border-gray-200 flex items-end gap-4">
            <div className="flex-1">
              <label className="block text-sm font-medium text-gray-700 mb-1">任务名称</label>
              <input
                type="text"
                value={newJobTitle}
                onChange={(e) => setNewJobTitle(e.target.value)}
                placeholder="例如：第一单元测试"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
              />
            </div>
            <div className="w-48">
              <label className="block text-sm font-medium text-gray-700 mb-1">批改模式</label>
              <select
                value={newJobMode}
                onChange={(e) => setNewJobMode(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow bg-white"
              >
                <option value="standard">标准批改</option>
                <option value="fast">快速批改</option>
              </select>
            </div>
            <button
              onClick={handleCreateJob}
              disabled={!newJobTitle.trim() || createJobMutation.isPending}
              className="flex items-center gap-2 px-5 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Check className="w-4 h-4" />
              确认创建
            </button>
          </div>
        )}
      </div>

      {/* 任务详情与操作区域 */}
      {selectedJob && (
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
          <div className="flex items-center justify-between mb-6">
            <div>
              <h2 className="text-lg font-semibold text-gray-900 flex items-center gap-3">
                {selectedJob.title}
                <span
                  className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                    JOB_STATUS_STYLES[selectedJob.status]?.bg || 'bg-gray-100'
                  } ${JOB_STATUS_STYLES[selectedJob.status]?.text || 'text-gray-600'}`}
                >
                  {JOB_STATUS_STYLES[selectedJob.status]?.label || selectedJob.status}
                </span>
              </h2>
              <p className="text-sm text-gray-500 mt-1">
                共 {selectedJob.total_assets} 份作业，已处理 {selectedJob.processed_assets} 份，失败 {selectedJob.failed_assets} 份，冲突 {selectedJob.conflict_count} 处
              </p>
            </div>
            <div className="flex items-center gap-3">
              <button
                onClick={() => {
                  setDeleteTargetId(selectedJob.id);
                  setDeleteTargetType('job');
                  setShowDeleteConfirm(true);
                }}
                disabled={isGrading}
                className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-red-600 bg-red-50 rounded-lg hover:bg-red-100 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Trash2 className="w-4 h-4" />
                删除任务
              </button>
              <button
                onClick={() => startGradingMutation.mutate(selectedJob.id)}
                disabled={isGrading || selectedJob.total_assets === 0 || selectedJob.status === 'processing'}
                className="flex items-center gap-2 px-5 py-2 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Play className="w-4 h-4" />
                {isGrading ? '批改中...' : '开始批改'}
              </button>
            </div>
          </div>

          {/* 进度条区域 */}
          {(isGrading || gradingProgress) && (
            <div className="mb-6 p-4 bg-gray-50 rounded-lg border border-gray-200">
              <div className="flex items-center justify-between text-sm mb-2">
                <span className="text-gray-700 font-medium">
                  {isGrading ? '正在批改作业...' : '批改完成'}
                </span>
                <span className="text-gray-500">{progressPercent}%</span>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-2.5 overflow-hidden mb-2">
                <div
                  className={`h-full rounded-full transition-all duration-500 ${
                    isGrading ? 'bg-brand-600' : 'bg-green-500'
                  }`}
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
              {gradingProgress && (
                <div className="flex items-center gap-4 text-sm text-gray-500">
                  {gradingProgress.current_filename && isGrading && (
                    <span>
                      正在处理: <span className="font-medium text-gray-700">{gradingProgress.current_filename}</span>
                    </span>
                  )}
                  <span>
                    已完成 {gradingProgress.processed}/{gradingProgress.total}
                  </span>
                  {gradingProgress.failed > 0 && (
                    <span className="flex items-center gap-1 text-amber-600">
                      <AlertTriangle className="w-3.5 h-3.5" />
                      {gradingProgress.failed} 个失败
                    </span>
                  )}
                  {gradingProgress.conflicts > 0 && (
                    <span className="flex items-center gap-1 text-red-600">
                      <AlertTriangle className="w-3.5 h-3.5" />
                      {gradingProgress.conflicts} 处冲突
                    </span>
                  )}
                </div>
              )}
            </div>
          )}

          {/* 拖拽上传区域 */}
          <div
            onDragOver={(e) => {
              e.preventDefault();
              setIsDragOver(true);
            }}
            onDragLeave={() => setIsDragOver(false)}
            onDrop={() => setIsDragOver(false)}
            className={`mb-6 transition-colors ${
              isDragOver
                ? 'border-2 border-solid border-brand-500 bg-brand-50 rounded-xl p-8 text-center'
                : 'border-2 border-dashed border-gray-300 rounded-xl p-8 text-center'
            }`}
          >
            <Upload className={`w-8 h-8 mx-auto mb-3 ${isDragOver ? 'text-brand-500' : 'text-gray-400'}`} />
            <p className="text-sm text-gray-600 font-medium">将作业图片拖拽到此处</p>
            <p className="text-xs text-gray-400 mt-1">支持 JPG、PNG 格式，自动识别学生信息</p>
          </div>

          {/* 素材列表 */}
          {jobAssets && jobAssets.length > 0 && (
            <div className="mb-6">
              <h3 className="text-sm font-medium text-gray-700 mb-3">已上传作业 ({jobAssets.length})</h3>
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
                {jobAssets.map((asset: AssignmentAsset) => (
                  <div key={asset.id} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-200">
                    <div className="truncate text-sm text-gray-700" title={asset.original_filename || asset.file_path}>
                      {asset.original_filename || asset.file_path.split(/[/\\]/).pop()}
                    </div>
                    <button
                      onClick={() => {
                        setDeleteTargetId(asset.id);
                        setDeleteTargetType('asset');
                        setShowDeleteConfirm(true);
                      }}
                      disabled={isGrading}
                      className="text-gray-400 hover:text-red-600 transition-colors disabled:opacity-50"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 冲突处理区域 */}
          {conflictResults && conflictResults.length > 0 && (
            <div className="mb-6 p-4 bg-red-50 rounded-lg border border-red-100">
              <h3 className="text-sm font-medium text-red-800 mb-3 flex items-center gap-2">
                <AlertTriangle className="w-4 h-4" />
                需要人工介入的冲突 ({conflictResults.length})
              </h3>
              <div className="space-y-3">
                {conflictResults.map((conflict: AssignmentOcrResult) => (
                  <div key={conflict.id} className="bg-white p-3 rounded border border-red-200 flex items-center justify-between">
                    <div className="text-sm">
                      <span className="font-medium text-gray-900 mr-3">{getStudentName(conflict.student_id)}</span>
                      <span className="text-gray-600 mr-3">题号: {conflict.question_no || '未知'}</span>
                      <span className="text-gray-600">识别结果: {conflict.answer_text || '空'}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handleReview(conflict.id, 'approved', conflict.score)}
                        className="px-3 py-1 text-xs font-medium text-green-700 bg-green-50 rounded hover:bg-green-100"
                      >
                        确认识别
                      </button>
                      <button
                        onClick={() => handleReview(conflict.id, 'rejected', 0)}
                        className="px-3 py-1 text-xs font-medium text-red-700 bg-red-50 rounded hover:bg-red-100"
                      >
                        标记错误
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 批改结果表格 */}
          {ocrResults && ocrResults.length > 0 && (
            <div>
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-medium text-gray-700">批改结果</h3>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleBatchApprove}
                    disabled={batchReviewMutation.isPending || ocrResults.filter((r) => r.review_status === 'pending').length === 0}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-green-700 bg-green-50 rounded-lg hover:bg-green-100 transition-colors disabled:opacity-50"
                  >
                    <Check className="w-4 h-4" />
                    批量确认
                  </button>
                  <button
                    onClick={handleExport}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
                  >
                    <FileSpreadsheet className="w-4 h-4" />
                    导出结果
                  </button>
                </div>
              </div>
              <div className="overflow-x-auto border border-gray-200 rounded-lg">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">学生</th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">题号</th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">答案文本</th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">置信度</th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">得分</th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">状态</th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">操作</th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {ocrResults.map((result: AssignmentOcrResult) => {
                      const statusStyle = REVIEW_STATUS_STYLES[result.review_status] || REVIEW_STATUS_STYLES.pending;
                      return (
                        <tr key={result.id} className="hover:bg-gray-50">
                          <td className="px-4 py-3 text-sm font-medium text-gray-900">{getStudentName(result.student_id)}</td>
                          <td className="px-4 py-3 text-sm text-gray-500">{result.question_no || '-'}</td>
                          <td className="px-4 py-3 text-sm text-gray-500 truncate max-w-[200px]" title={result.answer_text || ''}>
                            {result.answer_text || '-'}
                          </td>
                          <td className="px-4 py-3 text-sm text-gray-500">
                            {result.confidence ? `${(result.confidence * 100).toFixed(1)}%` : '-'}
                          </td>
                          <td className="px-4 py-3 text-sm font-medium text-gray-900">
                            {result.final_score !== null ? result.final_score : result.score !== null ? result.score : '-'}
                          </td>
                          <td className="px-4 py-3 text-sm">
                            <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${statusStyle.bg} ${statusStyle.text}`}>
                              {statusStyle.label}
                            </span>
                          </td>
                          <td className="px-4 py-3 text-sm font-medium">
                            {result.review_status === 'pending' && (
                              <div className="flex items-center gap-2">
                                <button
                                  onClick={() => handleReview(result.id, 'approved', result.score)}
                                  className="text-green-600 hover:text-green-900"
                                >
                                  确认
                                </button>
                                <button
                                  onClick={() => handleReview(result.id, 'rejected', 0)}
                                  className="text-red-600 hover:text-red-900"
                                >
                                  拒绝
                                </button>
                              </div>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      )}

      {/* 未选择任务时的空状态 */}
      {!selectedJob && !showCreateForm && (
        <EmptyState
          icon={<ClipboardCheck className="w-8 h-8" />}
          title="开始作业批改"
          description="选择班级并新建或选择一个批改任务，拖拽上传作业图片即可开始自动批改。"
        />
      )}

      {/* 删除确认对话框 */}
      <ConfirmDialog
        isOpen={showDeleteConfirm}
        title={deleteTargetType === 'job' ? '删除任务' : '删除素材'}
        message={deleteTargetType === 'job' ? '确定要删除该批改任务吗？相关的素材和批改结果也将被删除。' : '确定要删除该作业图片吗？'}
        confirmText="删除"
        onConfirm={() => {
          if (deleteTargetType === 'job' && deleteTargetId) {
            deleteJobMutation.mutate(deleteTargetId);
          } else if (deleteTargetType === 'asset' && deleteTargetId) {
            deleteAssetMutation.mutate(deleteTargetId);
          }
        }}
        onCancel={() => {
          setShowDeleteConfirm(false);
          setDeleteTargetId(null);
          setDeleteTargetType(null);
        }}
        isDestructive={true}
      />
    </div>
  );
};
