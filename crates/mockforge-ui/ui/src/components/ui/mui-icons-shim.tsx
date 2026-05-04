/* eslint-disable react-refresh/only-export-components */
/**
 * Compatibility shim for `@mui/icons-material`.
 *
 * Exports lucide-react icons under (a) their MUI name, (b) their MUI name
 * with `Icon` suffix, and (c) common alias names that legacy pages use.
 *
 * Lets us drop `@mui/icons-material` without rewriting every page in one
 * PR. If a needed MUI icon is missing here, add a mapping rather than
 * reaching back to `@mui/icons-material` — that package should not exist
 * after this migration.
 */
import * as L from 'lucide-react';

// MUI names that map to the same lucide-react name.
export const Search = L.Search;
export const SearchIcon = L.Search;
export const Save = L.Save;
export const SaveIcon = L.Save;
export const Send = L.Send;
export const SendIcon = L.Send;
export const Star = L.Star;
export const StarIcon = L.Star;
export const StarBorder = L.Star;
export const StarBorderIcon = L.Star;
export const Stars = L.Sparkles;
export const Code = L.Code;
export const CodeIcon = L.Code;
export const Download = L.Download;
export const DownloadIcon = L.Download;
export const Upload = L.Upload;
export const UploadIcon = L.Upload;
export const Settings = L.Settings;
export const SettingsIcon = L.Settings;
export const Menu = L.Menu;
export const MenuIcon = L.Menu;
export const Pause = L.Pause;
export const PauseIcon = L.Pause;
export const Mail = L.Mail;
export const MailIcon = L.Mail;
export const Email = L.Mail;
export const EmailIcon = L.Mail;
export const Key = L.Key;
export const KeyIcon = L.Key;
export const Lock = L.Lock;
export const LockIcon = L.Lock;
export const Timer = L.Timer;
export const TimerIcon = L.Timer;
export const TrendingUp = L.TrendingUp;
export const TrendingUpIcon = L.TrendingUp;
export const TrendingDown = L.TrendingDown;
export const Layers = L.Layers;
export const LayersIcon = L.Layers;
export const PlayCircle = L.PlayCircle;
export const PlayCircleIcon = L.PlayCircle;
export const StopCircle = L.StopCircle;
export const XCircle = L.XCircle;
export const CloudUpload = L.UploadCloud;
export const CloudUploadIcon = L.UploadCloud;
export const Zap = L.Zap;
export const Bolt = L.Zap;
export const BoltIcon = L.Zap;
export const CreditCard = L.CreditCard;
export const CreditCardIcon = L.CreditCard;

// MUI Add → lucide Plus
export const Add = L.Plus;
export const AddIcon = L.Plus;
export const Create = L.Plus;
export const PlusIcon = L.Plus;
export const Remove = L.Minus;
export const RemoveIcon = L.Minus;

// MUI ArrowBack → lucide ArrowLeft
export const ArrowBack = L.ArrowLeft;
export const ArrowBackIcon = L.ArrowLeft;
export const ArrowDownward = L.ArrowDown;
export const ArrowDownwardIcon = L.ArrowDown;
export const ArrowUpward = L.ArrowUp;
export const ArrowUpwardIcon = L.ArrowUp;
export const ChevronRight = L.ChevronRight;
export const ChevronRightIcon = L.ChevronRight;
export const KeyboardArrowRight = L.ChevronRight;
export const ArrowRight = L.ChevronRight;

// Open / Launch
export const Launch = L.ArrowUpRight;
export const LaunchIcon = L.ArrowUpRight;
export const LaunchOutlined = L.ArrowUpRight;
export const OpenInNew = L.ArrowUpRight;
export const OpenInNewIcon = L.ArrowUpRight;

// Errors / warnings / info
export const Error = L.AlertCircle;
export const ErrorIcon = L.AlertCircle;
export const ErrorOutline = L.AlertCircle;
export const ErrorOutlineIcon = L.AlertCircle;
export const Warning = L.AlertTriangle;
export const WarningIcon = L.AlertTriangle;
export const WarningAmber = L.AlertTriangle;
export const WarningAmberIcon = L.AlertTriangle;
export const Info = L.Info;
export const InfoIcon = L.Info;
export const InfoOutlined = L.Info;
export const InfoOutlinedIcon = L.Info;

// Charts / metrics
export const Assessment = L.BarChart3;
export const AssessmentIcon = L.BarChart3;
export const Analytics = L.BarChart3;
export const AnalyticsIcon = L.BarChart3;

// Misc
export const MenuBook = L.BookOpen;
export const MenuBookIcon = L.BookOpen;
export const BugReport = L.Bug;
export const BugReportIcon = L.Bug;
export const Check = L.Check;
export const CheckIcon = L.Check;
export const CheckCircle = L.CheckCircle;
export const CheckCircleIcon = L.CheckCircle;
export const SuccessIcon = L.CheckCircle;
export const Verified = L.CheckCircle;
export const VerifiedIcon = L.CheckCircle;
export const ExpandMore = L.ChevronDown;
export const ExpandMoreIcon = L.ChevronDown;
export const ExpandLess = L.ChevronUp;
export const ExpandLessIcon = L.ChevronUp;
export const FiberManualRecord = L.Circle;
export const FiberManualRecordIcon = L.Circle;
export const Schedule = L.Clock;
export const ScheduleIcon = L.Clock;
export const Pending = L.Clock;
export const PendingIcon = L.Clock;
export const Cloud = L.Cloud;
export const CloudIcon = L.Cloud;

// Copy / clipboard
export const ContentCopy = L.Copy;
export const ContentCopyIcon = L.Copy;
export const CopyIcon = L.Copy;
export const FileCopy = L.Copy;
export const Paste = L.Clipboard;
export const PasteIcon = L.Clipboard;

// Visibility
export const Visibility = L.Eye;
export const VisibilityIcon = L.Eye;
export const ViewIcon = L.Eye;
export const Watch = L.Eye;
export const WatchIcon = L.Eye;
export const VisibilityOff = L.EyeOff;
export const VisibilityOffIcon = L.EyeOff;

// Files / templates
export const Description = L.FileText;
export const DescriptionIcon = L.FileText;
export const TemplateIcon = L.FileText;
export const Article = L.FileText;
export const ArticleIcon = L.FileText;
export const FilterListIcon = L.Filter;
export const FilterList = L.Filter;
export const Movie = L.Film;
export const MovieIcon = L.Film;
export const Flag = L.Flag;
export const FlagIcon = L.Flag;
export const Folder = L.Folder;
export const FolderIcon = L.Folder;

// Categories / tags
export const Category = L.FolderTree;
export const CategoryIcon = L.FolderTree;

// Speed / metrics
export const Speed = L.Gauge;
export const SpeedIcon = L.Gauge;

// Git
export const AccountTree = L.GitBranch;
export const AccountTreeIcon = L.GitBranch;
export const BranchIcon = L.GitBranch;
export const Commit = L.GitCommit;
export const CommitIcon = L.GitCommit;
export const CompareArrows = L.GitCompare;
export const CompareArrowsIcon = L.GitCompare;
export const DiffIcon = L.GitCompare;
export const GitHub = L.Github;
export const GitHubIcon = L.Github;

// Globe / language
export const Language = L.Globe;
export const LanguageIcon = L.Globe;
export const Public = L.Globe;
export const PublicIcon = L.Globe;

// School
export const School = L.GraduationCap;
export const SchoolIcon = L.GraduationCap;

// Drag
export const DragIndicator = L.GripVertical;
export const DragIndicatorIcon = L.GripVertical;

// Hourglass / waiting
export const HourglassEmpty = L.Hourglass;
export const HourglassEmptyIcon = L.Hourglass;
export const HourglassTop = L.Hourglass;

// Login / logout
export const Login = L.LogIn;
export const LoginIcon = L.LogIn;
export const Logout = L.LogOut;
export const LogoutIcon = L.LogOut;

// More menu
export const MoreVert = L.MoreVertical;
export const MoreVertIcon = L.MoreVertical;
export const MoreHoriz = L.MoreHorizontal;
export const MoreHorizIcon = L.MoreHorizontal;

// Inventory / package
export const Inventory = L.Package;
export const InventoryIcon = L.Package;

// Edit / delete
export const Edit = L.Pencil;
export const EditIcon = L.Pencil;
export const Delete = L.Trash2;
export const DeleteIcon = L.Trash2;
export const DeleteForever = L.Trash;
export const DeleteForeverIcon = L.Trash;
export const YankIcon = L.Trash;

// Plays
export const PlayArrow = L.Play;
export const PlayArrowIcon = L.Play;
export const Stop = L.Square;
export const StopIcon = L.Square;
export const SkipNext = L.SkipForward;
export const SkipNextIcon = L.SkipForward;
export const SkipPrevious = L.SkipBack;
export const SkipPreviousIcon = L.SkipBack;

// Plugins
export const Extension = L.Puzzle;
export const ExtensionIcon = L.Puzzle;
export const PluginIcon = L.Puzzle;

// Refresh / sync
export const Refresh = L.RefreshCw;
export const RefreshIcon = L.RefreshCw;
export const Sync = L.RefreshCw;
export const SyncIcon = L.RefreshCw;
export const Autorenew = L.RotateCw;
export const AutorenewIcon = L.RotateCw;
export const RotateIcon = L.RotateCw;
export const Replay = L.RotateCcw;
export const ReplayIcon = L.RotateCcw;
export const Undo = L.RotateCcw;
export const UndoIcon = L.RotateCcw;
export const Restore = L.RotateCcw;
export const RestoreIcon = L.RotateCcw;
export const Redo = L.RotateCw;

// Publish / send
export const Publish = L.Send;
export const PublishIcon = L.Send;
export const Server = L.Server;
export const ServerIcon = L.Server;

// Security
export const Security = L.Shield;
export const SecurityIcon = L.Shield;
export const VerifiedUser = L.ShieldCheck;
export const VerifiedUserIcon = L.ShieldCheck;

// AI / sparkles
export const AutoFixHigh = L.Sparkles;
export const AutoFixHighIcon = L.Sparkles;
export const NewReleases = L.Sparkles;
export const NewReleasesIcon = L.Sparkles;
export const AutoMode = L.Wand2;

// Activity / timeline
export const Timeline = L.Activity;
export const TimelineIcon = L.Activity;

// Terminal
export const Terminal = L.TerminalSquare;
export const TerminalIcon = L.TerminalSquare;

// Thumbs
export const ThumbUp = L.ThumbsUp;
export const ThumbUpIcon = L.ThumbsUp;
export const ThumbUpAltOutlined = L.ThumbsUp;
export const ThumbUpAltOutlinedIcon = L.ThumbsUp;
export const ThumbDown = L.ThumbsDown;
export const ThumbDownIcon = L.ThumbsDown;

// File ops
export const FileDownload = L.Download;
export const FileDownloadIcon = L.Download;
export const FileUpload = L.Upload;
export const FileUploadIcon = L.Upload;
export const UploadFile = L.UploadCloud;
export const UploadFileIcon = L.UploadCloud;

// People
export const Person = L.User;
export const PersonIcon = L.User;
export const Group = L.Users;
export const GroupIcon = L.Users;
export const People = L.Users;
export const PeopleIcon = L.Users;

// Video
export const VideoLibrary = L.Video;
export const VideoLibraryIcon = L.Video;

// Close / cancel
export const Close = L.X;
export const CloseIcon = L.X;
export const Cancel = L.X;
export const CancelIcon = L.X;

// Dashboard
export const Dashboard = L.LayoutDashboard;
export const DashboardIcon = L.LayoutDashboard;

// Help
export const Help = L.HelpCircle;
export const HelpIcon = L.HelpCircle;
export const HelpOutline = L.HelpCircle;
export const HelpOutlineIcon = L.HelpCircle;
