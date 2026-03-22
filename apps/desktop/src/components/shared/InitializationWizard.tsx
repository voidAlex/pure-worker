/**
 * 首次启动初始化向导组件
 * 引导用户完成工作目录选择和 AI 供应商配置，完成前阻止使用主应用
 */
import React, { useState, useEffect } from 'react';
import {
  commands,
  type AppError,
  type ProviderPreset,
  type CreateAiConfigInput,
} from '@/services/commandClient';
import { TEACHING_STAGE_OPTIONS, TEACHING_SUBJECT_OPTIONS } from '@/constants/teacherProfile';
import {
  Bot,
  Sparkles,
  Zap,
  Cpu,
  Activity,
  FolderOpen,
  Eye,
  EyeOff,
  CheckCircle2,
  XCircle,
  Loader2,
  ArrowRight,
  ArrowLeft,
  Rocket,
  User,
} from 'lucide-react';

interface InitializationWizardProps {
  onComplete: () => void;
}

const PROVIDER_ICON_MAP: Record<string, React.ReactNode> = {
  openai: <Bot size={28} className="text-green-600" />,
  anthropic: <Sparkles size={28} className="text-purple-600" />,
  deepseek: <Zap size={28} className="text-blue-600" />,
  qwen: <Cpu size={28} className="text-orange-600" />,
  gemini: <Activity size={28} className="text-red-500" />,
};

const getErrorMessage = (err: AppError): string => {
  const values = Object.values(err as Record<string, string>);
  return values[0] ?? '未知错误';
};

const unwrapResult = <T,>(
  res: { status: 'ok'; data: T } | { status: 'error'; error: AppError },
): T => {
  if (res.status === 'ok') return res.data;
  throw new Error(getErrorMessage(res.error));
};

/** 模型信息接口 */
interface ModelInfo {
  id: string;
  name: string;
  is_vision: boolean;
}

export const InitializationWizard: React.FC<InitializationWizardProps> = ({ onComplete }) => {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [workspacePath, setWorkspacePath] = useState<string>('');

  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);

  const [apiKey, setApiKey] = useState<string>('');
  const [showApiKey, setShowApiKey] = useState<boolean>(false);
  const [connectionStatus, setConnectionStatus] = useState<
    'idle' | 'testing' | 'success' | 'failed'
  >('idle');
  const [connectionError, setConnectionError] = useState<string>('');
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string>('');

  // 教师信息
  const [teacherName, setTeacherName] = useState<string>('');
  const [teachingStage, setTeachingStage] = useState<string>('primary');
  const [teachingSubject, setTeachingSubject] = useState<string>('');

  const [saving, setSaving] = useState<boolean>(false);

  useEffect(() => {
    commands
      .getProviderPresets()
      .then((res) => {
        if (res.status === 'ok') {
          setPresets(res.data);
        }
      })
      .catch((err) => {
        console.error('获取预设供应商失败:', err);
      });
  }, []);

  const handleSelectDirectory = async () => {
    try {
      const res = await commands.selectDirectory();
      if (res.status === 'ok' && res.data) {
        setWorkspacePath(res.data);
      }
    } catch (err) {
      console.error('选择目录失败:', err);
    }
  };

  const handleTestConnection = async () => {
    if (!selectedPreset || !apiKey) return;

    setConnectionStatus('testing');
    setConnectionError('');
    setModels([]);
    setSelectedModelId('');

    try {
      const res = await commands.fetchProviderModels(
        selectedPreset.name,
        selectedPreset.base_url,
        apiKey,
      );
      const modelList = unwrapResult(res);

      setModels(modelList);
      if (modelList.length === 0) {
        setConnectionStatus('failed');
        setConnectionError('未发现可用模型，请检查 API Key 是否正确');
        return;
      }
      // 默认选择第一个模型
      setSelectedModelId(modelList[0].id);
      setConnectionStatus('success');
    } catch (error) {
      setConnectionStatus('failed');
      setConnectionError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleComplete = async () => {
    if (!selectedPreset || !apiKey || !workspacePath) return;

    setSaving(true);
    try {
      // 1. 保存工作区路径
      unwrapResult(
        await commands.updateSetting(
          'workspace_path',
          JSON.stringify(workspacePath),
          'general',
          '工作区目录路径',
        ),
      );

      // 2. 创建 AI 配置
      const aiConfig: CreateAiConfigInput = {
        provider_name: selectedPreset.name,
        display_name: selectedPreset.display_name,
        base_url: selectedPreset.base_url,
        api_key: apiKey,
        default_model: selectedModelId,
        default_text_model: null,
        default_vision_model: null,
        default_tool_model: null,
        default_reasoning_model: null,
        is_active: true,
        config_json: null,
      };
      unwrapResult(await commands.createAiConfig(aiConfig));

      // 3. 创建教师档案
      unwrapResult(
        await commands.createTeacherProfile({
          name: teacherName,
          teaching_stage: teachingStage,
          teaching_subject: teachingSubject,
        }),
      );

      // 4. 同步默认授课科目
      unwrapResult(
        await commands.updateSetting('default_subject', teachingSubject, 'general', '默认授课科目'),
      );

      // 5. 标记初始化完成
      unwrapResult(
        await commands.updateSetting(
          'initialization_completed',
          'true',
          'general',
          '初始化完成标记',
        ),
      );

      onComplete();
    } catch (error) {
      console.error('保存初始化设置失败:', error);
      alert(error instanceof Error ? error.message : '保存失败，请重试');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-gradient-to-br from-blue-50 via-white to-purple-50 flex items-center justify-center p-6">
      <div className="max-w-2xl w-full mx-auto bg-white rounded-xl shadow-xl border border-gray-200 overflow-hidden flex flex-col min-h-[600px] transition-all duration-300">
        {/* Header */}
        <div className="bg-brand-50 p-6 border-b border-brand-100 flex flex-col items-center justify-center">
          <div className="w-16 h-16 bg-brand-600 rounded-2xl flex items-center justify-center shadow-lg mb-4">
            <Rocket className="text-white" size={32} />
          </div>
          <h1 className="text-2xl font-bold text-gray-900">欢迎使用 PureWorker</h1>
          <p className="text-gray-500 mt-2">只需三步，即可开启您的智能教学助手之旅</p>
        </div>

        {/* Step Indicator */}
        <div className="flex items-center justify-center py-6 px-12 relative">
          <div className="flex items-center w-full max-w-lg relative z-10">
            {/* Step 1 */}
            <div className="flex flex-col items-center flex-1">
              <div
                className={`w-10 h-10 rounded-full flex items-center justify-center font-bold border-2 transition-colors duration-300
                ${step === 1 ? 'border-brand-600 bg-brand-50 text-brand-600' : step > 1 ? 'border-green-500 bg-green-500 text-white' : 'border-gray-200 bg-white text-gray-400'}`}
              >
                {step > 1 ? <CheckCircle2 size={20} /> : '1'}
              </div>
              <span
                className={`mt-2 text-sm font-medium ${step === 1 ? 'text-brand-700' : step > 1 ? 'text-gray-500' : 'text-gray-400'}`}
              >
                选择工作目录
              </span>
            </div>

            {/* Line 1-2 */}
            <div className="absolute top-5 left-[16.5%] w-[17%] h-0.5 bg-gray-200 -z-10">
              <div
                className="h-full bg-green-500 transition-all duration-500 ease-in-out"
                style={{ width: step > 1 ? '100%' : '0%' }}
              />
            </div>

            {/* Step 2 */}
            <div className="flex flex-col items-center flex-1">
              <div
                className={`w-10 h-10 rounded-full flex items-center justify-center font-bold border-2 transition-colors duration-300
                ${step === 2 ? 'border-brand-600 bg-brand-50 text-brand-600' : step > 2 ? 'border-green-500 bg-green-500 text-white' : 'border-gray-200 bg-white text-gray-400'}`}
              >
                {step > 2 ? <CheckCircle2 size={20} /> : '2'}
              </div>
              <span
                className={`mt-2 text-sm font-medium ${step === 2 ? 'text-brand-700' : 'text-gray-400'}`}
              >
                配置 AI 供应商
              </span>
            </div>

            {/* Line 2-3 */}
            <div className="absolute top-5 left-[49.5%] w-[17%] h-0.5 bg-gray-200 -z-10">
              <div
                className="h-full bg-green-500 transition-all duration-500 ease-in-out"
                style={{ width: step > 2 ? '100%' : '0%' }}
              />
            </div>

            {/* Step 3 */}
            <div className="flex flex-col items-center flex-1">
              <div
                className={`w-10 h-10 rounded-full flex items-center justify-center font-bold border-2 transition-colors duration-300
                ${step === 3 ? 'border-brand-600 bg-brand-50 text-brand-600' : 'border-gray-200 bg-white text-gray-400'}`}
              >
                3
              </div>
              <span
                className={`mt-2 text-sm font-medium ${step === 3 ? 'text-brand-700' : 'text-gray-400'}`}
              >
                填写教师信息
              </span>
            </div>
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 p-8 overflow-y-auto">
          {step === 1 && (
            <div className="flex flex-col items-center animate-in fade-in slide-in-from-bottom-4 duration-500">
              <FolderOpen size={64} className="text-brand-300 mb-6" />
              <p className="text-center text-gray-600 mb-8 max-w-md leading-relaxed">
                请选择一个文件夹作为 PureWorker 的工作区，所有教学资料和数据将保存在此目录中。
              </p>

              <div className="w-full max-w-md">
                <button
                  onClick={handleSelectDirectory}
                  className="w-full flex items-center justify-center py-3 px-4 border-2 border-dashed border-gray-300 rounded-lg hover:border-brand-500 hover:bg-brand-50 transition-colors duration-200 group"
                >
                  <FolderOpen className="mr-2 text-gray-400 group-hover:text-brand-500" size={20} />
                  <span className="font-medium text-gray-600 group-hover:text-brand-700">
                    选择目录
                  </span>
                </button>

                {workspacePath && (
                  <div className="mt-4 p-4 bg-gray-50 rounded-lg border border-gray-200 flex items-start">
                    <FolderOpen className="text-brand-500 mt-0.5 mr-3 flex-shrink-0" size={18} />
                    <div className="overflow-hidden">
                      <p className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-1">
                        当前选择的路径
                      </p>
                      <p className="text-sm text-gray-800 break-all font-mono bg-white p-2 rounded border border-gray-100">
                        {workspacePath}
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {step === 2 && (
            <div className="flex flex-col animate-in fade-in slide-in-from-bottom-4 duration-500">
              <h3 className="font-semibold text-gray-800 mb-4 flex items-center">
                <Bot className="mr-2 text-brand-500" size={20} />
                选择您偏好的 AI 供应商
              </h3>

              <div className="grid grid-cols-2 md:grid-cols-3 gap-3 mb-6">
                {presets.map((preset) => {
                  const isSelected = selectedPreset?.name === preset.name;
                  return (
                    <button
                      key={preset.name}
                      onClick={() => {
                        setSelectedPreset(preset);
                        setConnectionStatus('idle');
                        setConnectionError('');
                        setApiKey('');
                      }}
                      className={`flex flex-col items-center justify-center p-4 rounded-xl border-2 transition-all duration-200
                        ${isSelected ? 'border-brand-500 bg-brand-50 shadow-md ring-2 ring-brand-100' : 'border-gray-200 hover:border-brand-300 hover:bg-gray-50'}`}
                    >
                      {PROVIDER_ICON_MAP[preset.name] || (
                        <Bot size={28} className="text-gray-400" />
                      )}
                      <span className="mt-2 font-medium text-gray-800">{preset.display_name}</span>
                    </button>
                  );
                })}
              </div>

              {selectedPreset && (
                <div className="bg-gray-50 p-5 rounded-xl border border-gray-200 animate-in fade-in slide-in-from-top-2 duration-300">
                  <div className="mb-4">
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      API 密钥 (API Key)
                    </label>
                    <div className="relative">
                      <input
                        type={showApiKey ? 'text' : 'password'}
                        value={apiKey}
                        onChange={(e) => {
                          setApiKey(e.target.value);
                          setConnectionStatus('idle');
                        }}
                        className="w-full pl-3 pr-10 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent font-mono"
                        placeholder="sk-..."
                      />
                      <button
                        type="button"
                        onClick={() => setShowApiKey(!showApiKey)}
                        className="absolute right-3 top-2.5 text-gray-400 hover:text-gray-600 transition-colors"
                      >
                        {showApiKey ? <EyeOff size={18} /> : <Eye size={18} />}
                      </button>
                    </div>
                    <p className="mt-2 text-xs text-gray-500">
                      API 地址: {selectedPreset.base_url}
                    </p>
                  </div>

                  <div className="flex items-center space-x-3">
                    <button
                      onClick={handleTestConnection}
                      disabled={!apiKey || connectionStatus === 'testing'}
                      className="px-4 py-2 bg-white border border-gray-300 rounded-md text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 disabled:opacity-50 disabled:cursor-not-allowed flex items-center transition-colors"
                    >
                      {connectionStatus === 'testing' ? (
                        <>
                          <Loader2 className="animate-spin mr-2" size={16} /> 测试中
                        </>
                      ) : (
                        <>
                          <Activity className="mr-2" size={16} /> 测试连接
                        </>
                      )}
                    </button>

                    {connectionStatus === 'success' && (
                      <span className="flex items-center text-sm font-medium text-green-600 bg-green-50 px-3 py-1.5 rounded border border-green-100">
                        <CheckCircle2 className="mr-1" size={16} /> 连接成功 (发现 {models.length}{' '}
                        个模型)
                      </span>
                    )}
                  </div>

                  {/* 模型选择 - 连接成功后显示 */}
                  {connectionStatus === 'success' && models.length > 0 && (
                    <div className="mt-4 animate-in fade-in slide-in-from-top-2 duration-300">
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        选择默认模型
                      </label>
                      <select
                        value={selectedModelId}
                        onChange={(e) => setSelectedModelId(e.target.value)}
                        className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent"
                      >
                        {models.map((model) => (
                          <option key={model.id} value={model.id}>
                            {model.name}
                            {model.is_vision ? ' (支持视觉)' : ''}
                          </option>
                        ))}
                      </select>
                      <p className="mt-2 text-xs text-gray-500">
                        已选择{' '}
                        {models.find((m) => m.id === selectedModelId)?.name || selectedModelId}
                      </p>
                    </div>
                  )}

                  <div className="flex items-center space-x-3 mt-4">
                    {connectionStatus === 'failed' && (
                      <span className="flex items-center text-sm font-medium text-red-600 bg-red-50 px-3 py-1.5 rounded border border-red-100">
                        <XCircle className="mr-1 flex-shrink-0" size={16} />
                        <span className="truncate max-w-xs" title={connectionError}>
                          连接失败: {connectionError}
                        </span>
                      </span>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          {step === 3 && (
            <div className="flex flex-col animate-in fade-in slide-in-from-bottom-4 duration-500">
              <h3 className="font-semibold text-gray-800 mb-4 flex items-center">
                <User className="mr-2 text-brand-500" size={20} />
                填写您的教师信息
              </h3>
              <p className="text-gray-600 mb-6 text-sm">
                这些信息将帮助 PureWorker 更好地为您提供服务。
              </p>

              <div className="space-y-5">
                {/* 姓名 */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    您的姓名 <span className="text-red-500">*</span>
                  </label>
                  <input
                    type="text"
                    value={teacherName}
                    onChange={(e) => setTeacherName(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent"
                    placeholder="请输入您的姓名"
                  />
                </div>

                {/* 学段 */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    任教学段 <span className="text-red-500">*</span>
                  </label>
                  <select
                    value={teachingStage}
                    onChange={(e) => setTeachingStage(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent"
                  >
                    {TEACHING_STAGE_OPTIONS.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </div>

                {/* 学科 */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    任教学科 <span className="text-red-500">*</span>
                  </label>
                  <select
                    value={teachingSubject}
                    onChange={(e) => setTeachingSubject(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent"
                  >
                    <option value="">请选择学科</option>
                    {TEACHING_SUBJECT_OPTIONS.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-gray-100 p-6 bg-gray-50 flex justify-between rounded-b-xl">
          {step === 1 ? (
            <div /> // Placeholder for layout balance
          ) : (
            <button
              onClick={() => setStep((step - 1) as 1 | 2 | 3)}
              className="px-4 py-2 border border-gray-300 rounded-lg text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-brand-500 transition-colors flex items-center"
            >
              <ArrowLeft className="mr-2" size={16} />
              上一步
            </button>
          )}

          {step === 1 && (
            <button
              onClick={() => setStep(2)}
              disabled={!workspacePath}
              className="px-6 py-2 bg-brand-600 text-white rounded-lg text-sm font-medium hover:bg-brand-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center"
            >
              下一步
              <ArrowRight className="ml-2" size={16} />
            </button>
          )}

          {step === 2 && (
            <button
              onClick={() => setStep(3)}
              disabled={connectionStatus !== 'success'}
              className="px-6 py-2 bg-brand-600 text-white rounded-lg text-sm font-medium hover:bg-brand-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center"
            >
              下一步
              <ArrowRight className="ml-2" size={16} />
            </button>
          )}

          {step === 3 && (
            <button
              onClick={handleComplete}
              disabled={!teacherName.trim() || !teachingSubject || saving}
              className="px-6 py-2 bg-brand-600 text-white rounded-lg text-sm font-medium hover:bg-brand-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center"
            >
              {saving ? (
                <>
                  <Loader2 className="animate-spin mr-2" size={16} /> 保存中...
                </>
              ) : (
                <>
                  <CheckCircle2 className="mr-2" size={16} /> 完成设置
                </>
              )}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
