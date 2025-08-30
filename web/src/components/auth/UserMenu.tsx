/**
 * 用户菜单组件
 * 显示用户信息和认证相关操作
 */

import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import { useAuth } from '@/hooks/use-auth';
import { AuthRequired, AuthModeConditional, useAuthContext } from '@/components/AuthProvider';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { CompactLoginForm } from '@/components/auth/LoginForm';
import {
  User,
  LogOut,
  Settings,
  Key,
  Shield,
  LogIn,
  UserPlus,
  Crown,
} from 'lucide-react';

// ============================================================================
// 用户头像组件
// ============================================================================

interface UserAvatarProps {
  user: any;
  size?: 'sm' | 'md' | 'lg';
}

const UserAvatar: React.FC<UserAvatarProps> = ({ user, size = 'md' }) => {
  const sizeClasses = {
    sm: 'h-6 w-6',
    md: 'h-8 w-8',
    lg: 'h-10 w-10',
  };

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map(word => word[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  const displayName = user.display_name || user.username;
  const initials = getInitials(displayName);

  return (
    <Avatar className={sizeClasses[size]}>
      <AvatarImage src={user.avatar_url} alt={displayName} />
      <AvatarFallback className="text-xs font-medium">
        {initials}
      </AvatarFallback>
    </Avatar>
  );
};

// ============================================================================
// 认证状态指示器
// ============================================================================

const AuthStatusIndicator: React.FC = () => {
  const { authStatus } = useAuthContext();
  const { currentUser, isAuthenticated } = useAuth();

  if (!authStatus) return null;

  // 开放模式
  if (authStatus.auth_mode === 'open') {
    return (
      <Badge variant="secondary" className="text-xs">
        <Shield className="h-3 w-3 mr-1" />
        Open Mode
      </Badge>
    );
  }

  // 需要认证但未认证
  if (authStatus.auth_required && !isAuthenticated) {
    return (
      <Badge variant="destructive" className="text-xs">
        Authentication Required
      </Badge>
    );
  }

  // 已认证
  if (isAuthenticated && currentUser) {
    return (
      <div className="flex items-center gap-2">
        <UserAvatar user={currentUser} size="sm" />
        <div className="flex flex-col">
          <span className="text-sm font-medium">
            {currentUser.display_name || currentUser.username}
          </span>
          {currentUser.is_admin && (
            <Badge variant="default" className="text-xs w-fit">
              <Crown className="h-3 w-3 mr-1" />
              Admin
            </Badge>
          )}
        </div>
      </div>
    );
  }

  return null;
};

// ============================================================================
// 用户菜单组件
// ============================================================================

export const UserMenu: React.FC = () => {
  const { authStatus } = useAuthContext();
  const { currentUser, isAuthenticated, logout } = useAuth();
  const [showLoginDialog, setShowLoginDialog] = useState(false);

  // 开放模式，不显示用户菜单
  if (authStatus?.auth_mode === 'open') {
    return (
      <div className="flex items-center gap-2">
        <AuthStatusIndicator />
      </div>
    );
  }

  // 需要认证但未认证
  if (authStatus?.auth_required && !isAuthenticated) {
    return (
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowLoginDialog(true)}
        >
          <LogIn className="h-4 w-4 mr-2" />
          Sign In
        </Button>
        <Button variant="default" size="sm" asChild>
          <Link to="/register">
            <UserPlus className="h-4 w-4 mr-2" />
            Sign Up
          </Link>
        </Button>

        {/* 登录对话框 */}
        <Dialog open={showLoginDialog} onOpenChange={setShowLoginDialog}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Sign In to Wikify</DialogTitle>
            </DialogHeader>
            <CompactLoginForm
              onSuccess={() => setShowLoginDialog(false)}
              onCancel={() => setShowLoginDialog(false)}
            />
          </DialogContent>
        </Dialog>
      </div>
    );
  }

  // 已认证，显示用户菜单
  if (isAuthenticated && currentUser) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" className="relative h-8 w-8 rounded-full">
            <UserAvatar user={currentUser} size="md" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent className="w-56" align="end" forceMount>
          <DropdownMenuLabel className="font-normal">
            <div className="flex flex-col space-y-1">
              <p className="text-sm font-medium leading-none">
                {currentUser.display_name || currentUser.username}
              </p>
              {currentUser.email && (
                <p className="text-xs leading-none text-muted-foreground">
                  {currentUser.email}
                </p>
              )}
              <div className="flex items-center gap-2 mt-2">
                <Badge variant="secondary" className="text-xs">
                  {currentUser.user_type}
                </Badge>
                {currentUser.is_admin && (
                  <Badge variant="default" className="text-xs">
                    <Crown className="h-3 w-3 mr-1" />
                    Admin
                  </Badge>
                )}
              </div>
            </div>
          </DropdownMenuLabel>
          <DropdownMenuSeparator />
          
          <DropdownMenuItem asChild>
            <Link to="/profile">
              <User className="mr-2 h-4 w-4" />
              <span>Profile</span>
            </Link>
          </DropdownMenuItem>
          
          <DropdownMenuItem asChild>
            <Link to="/settings">
              <Settings className="mr-2 h-4 w-4" />
              <span>Settings</span>
            </Link>
          </DropdownMenuItem>

          {/* API Keys - 仅在企业模式下显示 */}
          <AuthModeConditional modes={['enterprise']}>
            <DropdownMenuItem asChild>
              <Link to="/api-keys">
                <Key className="mr-2 h-4 w-4" />
                <span>API Keys</span>
              </Link>
            </DropdownMenuItem>
          </AuthModeConditional>

          <DropdownMenuSeparator />
          
          <DropdownMenuItem
            className="text-destructive focus:text-destructive"
            onClick={logout}
          >
            <LogOut className="mr-2 h-4 w-4" />
            <span>Sign Out</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }

  return null;
};

// ============================================================================
// 简化版用户信息显示
// ============================================================================

export const UserInfo: React.FC = () => {
  const { authStatus } = useAuthContext();
  const { currentUser, isAuthenticated } = useAuth();

  if (!authStatus) return null;

  return (
    <div className="flex items-center gap-2">
      <AuthStatusIndicator />
    </div>
  );
};

export default UserMenu;
