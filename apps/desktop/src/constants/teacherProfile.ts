export interface TeacherStageOption {
  value: string;
  label: string;
}

export interface TeacherSubjectOption {
  value: string;
  label: string;
}

export const TEACHING_STAGE_OPTIONS: TeacherStageOption[] = [
  { value: 'primary', label: '小学' },
  { value: 'junior', label: '初中' },
  { value: 'senior', label: '高中' },
];

export const TEACHING_SUBJECT_OPTIONS: TeacherSubjectOption[] = [
  { value: '语文', label: '语文' },
  { value: '数学', label: '数学' },
  { value: '英语', label: '英语' },
  { value: '物理', label: '物理' },
  { value: '化学', label: '化学' },
  { value: '生物', label: '生物' },
  { value: '历史', label: '历史' },
  { value: '地理', label: '地理' },
  { value: '政治', label: '政治' },
  { value: '音乐', label: '音乐' },
  { value: '美术', label: '美术' },
  { value: '体育', label: '体育' },
  { value: '信息技术', label: '信息技术' },
  { value: '其他', label: '其他' },
];
