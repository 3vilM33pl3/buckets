# Buckets CLI v0.2.0 Release Notes

**Release Date:** September 6, 2025  
**Release Type:** Major Release

## 🎉 Overview

Buckets v0.2.0 represents a significant milestone in the evolution of our game asset and expectation management CLI tool. This major release introduces fundamental architectural improvements, enhanced configuration management, and comprehensive documentation updates that make Buckets more robust, user-friendly, and production-ready.

## ⭐ Major Features

### 🔧 Global Configuration System
- **NEW**: `buckets setup` command for global configuration management
- Create and manage global configuration files (`~/.buckets_config.toml`)
- Interactive setup for PostgreSQL connection strings and NTP server configuration
- Automatic inheritance of global settings by local repositories
- Graceful error handling and configuration recovery

### 🗃️ Database Architecture Overhaul
- **BREAKING**: Complete migration from DuckDB to PostgreSQL
- Support for both embedded PostgreSQL (default) and external PostgreSQL instances
- Improved database initialization and schema management
- Enhanced connection handling with better error messages
- Automatic database setup with proper permissions

### 📦 Enhanced Packaging & Distribution
- **NEW**: Debian package support via `cargo-deb`
- Improved CI/CD pipeline with automated releases
- Better artifact management and distribution
- Cross-platform compatibility improvements

## 🚀 New Features

### Setup Command (`buckets setup`)
```bash
# Interactive global configuration setup
buckets setup

# Configure PostgreSQL connection and NTP server
# Automatically inherited by new repositories
```

**Key Capabilities:**
- Global PostgreSQL connection string configuration
- NTP server customization
- Configuration persistence and validation
- Error recovery from corrupted config files
- Works outside any buckets repository

### Database Flexibility
- **Embedded PostgreSQL** (default): Zero-configuration local database
- **External PostgreSQL**: Connect to existing PostgreSQL servers
- Automatic database type detection and validation
- Improved connection string handling

### Repository Configuration Inheritance
- New repositories automatically inherit global configuration
- PostgreSQL connection strings propagated from global setup
- NTP server settings shared across all repositories
- Backward compatibility maintained

## 🔧 Improvements

### Documentation Enhancements
- **Completely reorganized manual testing documentation** (27 comprehensive test cases)
- Sequential test numbering (TC001-TC027) for better organization
- Setup command tests prioritized first for logical testing workflow
- Table of contents with navigation links
- Command-based test categorization
- Updated automation scripts and reporting templates

### Code Quality & Maintenance
- Eliminated compilation warnings
- Removed obsolete DuckDB-era code and functions
- Improved external database validation
- Enhanced error handling and user feedback
- Better test coverage and reliability

### CI/CD Improvements
- Enhanced GitHub Actions workflows
- Automated release creation on tag pushes
- Improved artifact handling
- Better secret management

## 🔥 Breaking Changes

### Database Migration
- **REQUIRED**: Existing repositories using DuckDB must be migrated
- Database files and connection methods have changed
- External database configuration syntax updated

### Configuration Changes
- New global configuration file location (`~/.buckets_config.toml`)
- Repository configuration format updated to support inheritance
- Some command-line flags may have changed

### Migration Guide
1. **Backup existing repositories** before upgrading
2. **Run `buckets setup`** to configure global settings
3. **Re-initialize repositories** if database migration issues occur
4. **Update external database connections** using new configuration format

## 🐛 Bug Fixes

- Fixed external database configuration to fail gracefully instead of defaulting
- Resolved PostgreSQL connection issues with rate limiting
- Fixed configuration inheritance bugs for PostgreSQL connections
- Improved error handling for invalid database types
- Fixed test failures related to network dependencies
- Resolved Debian packaging configuration issues

## 📚 Documentation

### New Documentation
- **Setup Command Documentation** (`docs/setup_command.md`)
- **Manual Testing Plan** reorganization with 27 test cases
- **Configuration Management Guide**
- **Database Migration Instructions**

### Updated Documentation
- **README.md** with setup command instructions
- **Architecture documentation** reflecting PostgreSQL changes
- **Development commands** updated for new build process
- **Testing infrastructure** documentation

## 🔬 Testing

### Enhanced Test Suite
- **27 comprehensive test cases** (TC001-TC027)
- **Setup command test priority** - run first for proper workflow
- **Configuration inheritance testing** (TC003)
- **Cross-platform compatibility tests**
- **Error handling and edge case coverage**
- **Performance and load testing scenarios**

### Test Categories
1. **Setup Command Tests** (TC001-TC004) - Critical foundation
2. **Core Functionality Tests** (TC005-TC014) - Essential operations  
3. **Information Command Tests** (TC015-TC018) - Data retrieval
4. **Expectation Management Tests** (TC019-TC022) - Project workflow
5. **Database Management Tests** (TC023) - Schema operations
6. **Integration & Performance Tests** (TC024-TC025) - Complex scenarios
7. **Error Handling Tests** (TC026-TC027) - Edge cases and help

### Automation Improvements
- Updated automation scripts reflect setup command priority
- Integration test validates configuration inheritance
- Performance benchmarks maintained
- Cleanup scripts handle global configuration files

## 💻 Development

### New Dependencies
- `dirs = "5.0"` - Home directory access for global configuration
- Enhanced PostgreSQL support libraries
- Improved TOML handling for configuration files

### Removed Dependencies
- All DuckDB-related dependencies eliminated
- Obsolete database connection libraries removed
- Cleaned up unused testing dependencies

### Build Improvements
- Cargo-deb integration for Debian packages
- Improved cross-compilation support
- Better artifact generation in CI/CD

## 🚦 Upgrade Instructions

### For Users
1. **Install v0.2.0** using your preferred method
2. **Run initial setup**: `buckets setup`
3. **Configure global settings** when prompted
4. **Test existing repositories** to ensure compatibility
5. **Re-initialize repositories** if database issues occur

### For Developers
1. **Update development environment** with new dependencies
2. **Run new test suite** following TC001-TC027 sequence
3. **Update CI/CD configurations** if using custom pipelines
4. **Review documentation changes** for API updates

## 📊 Statistics

- **Commits in this release:** 45+
- **Pull requests merged:** 7 major PRs
- **Test cases:** 27 comprehensive scenarios
- **Documentation files updated:** 8+
- **Commands fully tested:** 16 (100% coverage)

## 🙏 Acknowledgments

This release represents significant architectural improvements and enhanced user experience. Special recognition for:

- **Database migration** from DuckDB to PostgreSQL
- **Global configuration system** design and implementation
- **Comprehensive documentation** reorganization
- **Testing infrastructure** improvements
- **CI/CD pipeline** enhancements

## 🔮 Looking Forward

v0.2.0 establishes a solid foundation for future development with:
- Scalable database architecture
- Flexible configuration management
- Comprehensive testing framework
- Enhanced documentation structure
- Improved development workflow

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/3vilM33pl3/buckets/issues)
- **Documentation**: See `docs/` directory
- **Testing**: Follow manual test plan (TC001-TC027)
- **Configuration**: Use `buckets setup --help` for guidance

---

**Full Changelog**: [v0.1.6...v0.2.0](https://github.com/3vilM33pl3/buckets/compare/v0.1.6...v0.2.0)

**Download**: [Release Page](https://github.com/3vilM33pl3/buckets/releases/tag/v0.2.0)