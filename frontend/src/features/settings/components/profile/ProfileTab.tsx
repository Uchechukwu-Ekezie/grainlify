import { useState, useEffect, useRef } from 'react';
import { Github, User, Upload, Link as LinkIcon } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { getCurrentUser, updateProfile, updateAvatar } from '../../../../shared/api/client';

interface CurrentUser {
  id: string;
  role: string;
  first_name?: string;
  last_name?: string;
  github?: {
    login: string;
    avatar_url: string;
    name?: string;
    email?: string;
    location?: string;
    bio?: string;
    website?: string;
  };
}

export function ProfileTab() {
  const { theme } = useTheme();
  const [currentUser, setCurrentUser] = useState<CurrentUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  
  // Form state
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [location, setLocation] = useState('');
  const [website, setWebsite] = useState('');
  const [bio, setBio] = useState('');
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const [telegram, setTelegram] = useState('');
  const [linkedin, setLinkedin] = useState('');
  const [whatsapp, setWhatsapp] = useState('');
  const [twitter, setTwitter] = useState('');
  const [discord, setDiscord] = useState('');

  useEffect(() => {
    const fetchUser = async () => {
      setIsLoading(true);
      try {
        const user = await getCurrentUser();
        setCurrentUser(user);
        
        // Prefill form fields from database (preferred) or GitHub data
        // Use database values first, then fallback to GitHub
        if (user.first_name) {
          setFirstName(user.first_name);
        } else if (user.github?.name) {
          const nameParts = user.github.name.trim().split(/\s+/);
          if (nameParts.length > 0) {
            setFirstName(nameParts[0]);
          }
        }
        
        if (user.last_name) {
          setLastName(user.last_name);
        } else if (user.github?.name) {
          const nameParts = user.github.name.trim().split(/\s+/);
          if (nameParts.length > 1) {
            setLastName(nameParts.slice(1).join(' '));
          }
        }
        
        if (user.github) {
          // Set avatar URL (database avatar_url takes precedence)
          if (user.github.avatar_url) {
            setAvatarUrl(user.github.avatar_url);
          }
          
          // Use database values if available, otherwise use GitHub
          if (user.github.location) {
            setLocation(user.github.location);
          }
          if (user.github.website) {
            setWebsite(user.github.website);
          }
          if (user.github.bio) {
            setBio(user.github.bio);
          }
        }
      } catch (error) {
        console.error('Failed to fetch user data:', error);
      } finally {
        setIsLoading(false);
      }
    };
    fetchUser();
  }, []);

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // Validate file type
    const validTypes = ['image/svg+xml', 'image/png', 'image/jpeg', 'image/jpg', 'image/gif'];
    if (!validTypes.includes(file.type)) {
      alert('Please select a valid image file (SVG, PNG, JPG, or GIF)');
      return;
    }

    // Validate file size (max 5MB)
    if (file.size > 5 * 1024 * 1024) {
      alert('File size must be less than 5MB');
      return;
    }

    // Convert to base64 data URL for preview
    const reader = new FileReader();
    reader.onloadend = () => {
      const base64String = reader.result as string;
      setAvatarUrl(base64String);
      // Don't upload yet - wait for user to click "Save Picture" button
    };
    reader.readAsDataURL(file);
  };

  const handleSaveAvatar = async () => {
    if (!avatarUrl) return;
    
    setIsSaving(true);
    try {
      await updateAvatar(avatarUrl);
      // Refetch user data to get updated avatar
      const user = await getCurrentUser();
      setCurrentUser(user);
      if (user.github?.avatar_url) {
        setAvatarUrl(user.github.avatar_url);
      }
      alert('Profile picture updated successfully!');
    } catch (error) {
      console.error('Failed to update avatar:', error);
      alert('Failed to update avatar. Please try again.');
    } finally {
      setIsSaving(false);
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await updateProfile({
        first_name: firstName || undefined,
        last_name: lastName || undefined,
        location: location || undefined,
        website: website || undefined,
        bio: bio || undefined,
        telegram: telegram || undefined,
        linkedin: linkedin || undefined,
        whatsapp: whatsapp || undefined,
        twitter: twitter || undefined,
        discord: discord || undefined,
      });
      // Refetch user data to get updated profile
      const user = await getCurrentUser();
      setCurrentUser(user);
      
      // Update form fields with saved data from database
      setFirstName(user.first_name || '');
      setLastName(user.last_name || '');
      if (user.github) {
        setLocation(user.github.location || '');
        setWebsite(user.github.website || '');
        setBio(user.github.bio || '');
        if (user.github.avatar_url) {
          setAvatarUrl(user.github.avatar_url);
        }
      }
      
      alert('Profile updated successfully!');
    } catch (error) {
      console.error('Failed to update profile:', error);
      alert('Failed to update profile. Please try again.');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Profile Header */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <h2 className={`text-[28px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}>Profile</h2>
        <p className={`text-[14px] transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>You can edit all your information here.</p>
      </div>

      {/* GitHub Account Section */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <h3 className={`text-[20px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}>GitHub account</h3>
        <p className={`text-[14px] mb-6 transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>
          To change your username or email, edit your account on Github, then resync your account.
        </p>

        <div className={`flex items-center justify-between p-4 rounded-[16px] backdrop-blur-[30px] border transition-colors ${
          theme === 'dark'
            ? 'bg-[#3d342c]/[0.4] border-white/15'
            : 'bg-white/[0.15] border-white/25'
        }`}>
          <span className={`text-[15px] font-medium transition-colors ${
            theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#2d2820]'
          }`}>
            {isLoading ? (
              <span className="inline-block w-32 h-4 bg-white/10 rounded animate-pulse" />
            ) : currentUser?.github ? (
              `${currentUser.github.login} / ${currentUser.github.email || `${currentUser.github.login}@users.noreply.github.com`}`
            ) : (
              'Not connected / Not connected'
            )}
          </span>
          <div className="flex items-center gap-3">
            <button className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] hover:bg-white/[0.25] transition-all flex items-center gap-2 ${
              theme === 'dark'
                ? 'bg-[#3d342c]/[0.5] border-white/20 text-[#d4c5b0]'
                : 'bg-white/[0.2] border-white/30 text-[#2d2820]'
            }`}>
              <Github className="w-4 h-4" />
              Resync
            </button>
            <button className="px-5 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[14px] shadow-[0_4px_16px_rgba(162,121,44,0.3)] hover:shadow-[0_6px_20px_rgba(162,121,44,0.4)] transition-all border border-white/10">
              Edit
            </button>
          </div>
        </div>
      </div>

      {/* Profile Picture */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <h3 className={`text-[16px] font-bold mb-1 transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}>Profile Picture</h3>
        <p className={`text-[13px] mb-5 transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>SVG, PNG, JPG or GIF</p>

        <div className="flex items-center gap-4">
          {isLoading ? (
            <div className="w-16 h-16 rounded-full bg-white/10 animate-pulse" />
          ) : avatarUrl ? (
            <img
              src={avatarUrl}
              alt="Profile"
              className="w-16 h-16 rounded-full object-cover shadow-md border border-white/15"
            />
          ) : (
            <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md border border-white/15">
              <User className="w-8 h-8 text-white" />
            </div>
          )}
          <input
            type="file"
            ref={fileInputRef}
            onChange={handleFileUpload}
            accept="image/svg+xml,image/png,image/jpeg,image/jpg,image/gif"
            className="hidden"
          />
          <button
            onClick={() => fileInputRef.current?.click()}
            className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] hover:bg-white/[0.2] transition-all flex items-center gap-2 ${
              theme === 'dark'
                ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#d4c5b0]'
                : 'bg-white/[0.15] border-white/25 text-[#2d2820]'
            }`}
          >
            <Upload className="w-4 h-4" />
            Update
          </button>
          {avatarUrl && avatarUrl !== currentUser?.github?.avatar_url && (
            <button
              onClick={handleSaveAvatar}
              disabled={isSaving}
              className={`px-5 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[14px] shadow-[0_4px_16px_rgba(162,121,44,0.3)] hover:shadow-[0_6px_20px_rgba(162,121,44,0.4)] transition-all border border-white/10 disabled:opacity-50 disabled:cursor-not-allowed`}
            >
              {isSaving ? 'Saving...' : 'Save Picture'}
            </button>
          )}
        </div>
      </div>

      {/* Personal Information */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* First Name */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>First Name</label>
            <input
              type="text"
              placeholder="Enter your first name"
              value={firstName}
              onChange={(e) => setFirstName(e.target.value)}
              className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                theme === 'dark'
                  ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                  : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
              }`}
            />
          </div>

          {/* Last Name */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Last Name</label>
            <input
              type="text"
              placeholder="Enter your last name"
              value={lastName}
              onChange={(e) => setLastName(e.target.value)}
              className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                theme === 'dark'
                  ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                  : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
              }`}
            />
          </div>

          {/* Location */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Location</label>
            <input
              type="text"
              placeholder="Enter your location"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                theme === 'dark'
                  ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                  : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
              }`}
            />
          </div>

          {/* Website */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Website</label>
            <input
              type="text"
              placeholder="Enter your website"
              value={website}
              onChange={(e) => setWebsite(e.target.value)}
              className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                theme === 'dark'
                  ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                  : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
              }`}
            />
          </div>
        </div>

        {/* Bio */}
        <div className="mt-6">
          <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>Bio</label>
          <textarea
            placeholder="Enter your bio"
            rows={4}
            value={bio}
            onChange={(e) => setBio(e.target.value)}
            className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] resize-none ${
              theme === 'dark'
                ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
            }`}
          />
        </div>
      </div>

      {/* Contact Information */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <h3 className={`text-[20px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}>Contact Information</h3>
        <p className={`text-[14px] mb-6 transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>
          Please enter only your social networks handle (no links, no @ needed).
        </p>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Telegram */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Telegram</label>
            <div className="relative">
              <input
                type="text"
                placeholder="Enter your telegram handle"
                className={`w-full px-4 py-3 pr-10 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                  theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
              />
              <LinkIcon className={`absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
                theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
              }`} />
            </div>
          </div>

          {/* LinkedIn */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>LinkedIn</label>
            <div className="relative">
              <input
                type="text"
                value={linkedin}
                onChange={(e) => setLinkedin(e.target.value)}
                placeholder="Enter your linkedin handle"
                className={`w-full px-4 py-3 pr-10 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                  theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
              />
              <LinkIcon className={`absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
                theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
              }`} />
            </div>
          </div>

          {/* WhatsApp */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>WhatsApp</label>
            <div className="relative">
              <input
                type="text"
                value={whatsapp}
                onChange={(e) => setWhatsapp(e.target.value)}
                placeholder="Enter your whatsApp handle"
                className={`w-full px-4 py-3 pr-10 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                  theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
              />
              <LinkIcon className={`absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
                theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
              }`} />
            </div>
          </div>

          {/* Twitter */}
          <div>
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Twitter</label>
            <div className="relative">
              <input
                type="text"
                value={twitter}
                onChange={(e) => setTwitter(e.target.value)}
                placeholder="Enter your twitter handle"
                className={`w-full px-4 py-3 pr-10 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                  theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
              />
              <LinkIcon className={`absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
                theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
              }`} />
            </div>
          </div>

          {/* Discord - Full Width */}
          <div className="md:col-span-2">
            <label className={`block text-[14px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Discord</label>
            <div className="relative">
              <input
                type="text"
                value={discord}
                onChange={(e) => setDiscord(e.target.value)}
                placeholder="Enter your discord handle"
                className={`w-full px-4 py-3 pr-10 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all text-[14px] ${
                  theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
              />
              <LinkIcon className={`absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
                theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
              }`} />
            </div>
          </div>
        </div>
      </div>

      {/* Save Button */}
      <div className="flex justify-end">
        <button
          onClick={handleSave}
          disabled={isSaving}
          className={`px-8 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10 disabled:opacity-50 disabled:cursor-not-allowed`}
        >
          {isSaving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  );
}