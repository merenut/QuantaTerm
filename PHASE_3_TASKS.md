# Phase 3 Task Breakdown: Plugins & AI (Weeks 15-22)

## Overview
This document provides a detailed breakdown of Phase 3 tasks from the PROJECT_PLAN.md, specifically designed for AI coding agents. Each task includes clear acceptance criteria, implementation guidance, and testing requirements.

**Phase 3 Goals**: Implement extensible plugin system and AI integration
**Duration**: 8 weeks (Weeks 15-22)
**Dependencies**: Phase 0-2 foundations (PTY, rendering, config system)

---

## Task 1: WASM Runtime (Wasmtime Host Environment)

### **1.1 WASM Host Environment Setup**
**Priority**: Critical
**Estimated Time**: 5-6 days
**Dependencies**: None

#### Requirements
- Integrate Wasmtime as the WASM runtime engine
- Create plugin host environment with resource limits
- Implement module loading and instantiation
- Support both file-based and embedded plugins
- Memory and execution time budgeting

#### Implementation Guidelines
```rust
// File: crates/plugins-host/src/runtime.rs
pub struct WasmRuntime {
    engine: wasmtime::Engine,
    store: wasmtime::Store<HostContext>,
    linker: wasmtime::Linker<HostContext>,
    execution_limits: ExecutionLimits,
}

// File: crates/plugins-host/src/host_context.rs
pub struct HostContext {
    capabilities: CapabilitySet,
    memory_limit: u64,
    time_limit: Duration,
}

// File: crates/plugins-host/src/limits.rs
pub struct ExecutionLimits {
    max_memory: u64,       // 16MB default
    max_time: Duration,    // 100ms default
    max_fuel: u64,         // Computation limit
}
```

#### Acceptance Criteria
- [ ] Load example WASM plugin from file
- [ ] Execute plugin with memory limit (16MB default)
- [ ] Abort plugin execution after timeout (100ms default)
- [ ] Plugin isolation - no access to host filesystem by default
- [ ] Handle plugin crashes gracefully without UI freeze

#### Test Requirements
```rust
#[test]
fn test_wasm_plugin_loading() {
    let runtime = WasmRuntime::new().unwrap();
    let plugin = runtime.load_plugin("test_plugin.wasm").unwrap();
    assert!(plugin.is_valid());
}

#[test]
fn test_execution_timeout() {
    let runtime = WasmRuntime::new().unwrap();
    let start = Instant::now();
    let result = runtime.execute_with_timeout(Duration::from_millis(100));
    assert!(start.elapsed() <= Duration::from_millis(150));
    assert!(result.is_err()); // Should timeout
}
```

### **1.2 Plugin Loading and Module Management**
**Priority**: Critical
**Estimated Time**: 3-4 days
**Dependencies**: 1.1

#### Requirements
- Plugin discovery from configured directories
- Manifest validation (plugin.toml)
- Module caching and hot-reload support
- Plugin versioning and compatibility checks

#### Implementation Guidelines
```rust
// File: crates/plugins-host/src/loader.rs
pub struct PluginLoader {
    plugin_dirs: Vec<PathBuf>,
    loaded_plugins: HashMap<String, LoadedPlugin>,
    watcher: Option<notify::RecommendedWatcher>,
}

// File: crates/plugins-host/src/manifest.rs
#[derive(Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry_point: String,
    pub capabilities: Vec<String>,
    pub quantaterm_version: String,
}
```

#### Acceptance Criteria
- [ ] Discover plugins in ~/.config/quantaterm/plugins/
- [ ] Validate plugin.toml manifest before loading
- [ ] Load multiple plugins simultaneously
- [ ] Detect plugin file changes and support hot-reload
- [ ] Reject plugins with incompatible version requirements

---

## Task 2: Capability System (Manifest Permissions)

### **2.1 Capability Framework**
**Priority**: Critical
**Estimated Time**: 4-5 days
**Dependencies**: 1.1

#### Requirements
- Define capability types (fs.read, fs.write, net.fetch, block.read, palette.addAction)
- Manifest-based permission declaration
- Runtime permission enforcement
- Capability inheritance and delegation

#### Implementation Guidelines
```rust
// File: crates/plugins-host/src/capabilities.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    FileSystemRead(PathPattern),
    FileSystemWrite(PathPattern),
    NetworkFetch(UrlPattern),
    BlockRead,
    BlockWrite,
    PaletteAddAction,
    ConfigRead,
    ConfigWrite,
}

#[derive(Debug, Clone)]
pub struct CapabilitySet {
    capabilities: HashSet<Capability>,
    plugin_id: String,
}

// File: crates/plugins-host/src/permission_check.rs
pub trait PermissionChecker {
    fn check_permission(&self, plugin_id: &str, capability: &Capability) -> Result<(), PermissionError>;
    fn grant_capability(&mut self, plugin_id: &str, capability: Capability);
    fn revoke_capability(&mut self, plugin_id: &str, capability: &Capability);
}
```

#### Acceptance Criteria
- [ ] Plugin with fs.read capability can read files
- [ ] Plugin without fs.read capability fails safely on file access
- [ ] Unauthorized filesystem access throws PermissionError
- [ ] Capability checks add < 1ms overhead per operation
- [ ] Capability grants are persistent across plugin reloads

#### Test Requirements
```rust
#[test]
fn test_unauthorized_fs_access_fails() {
    let mut runtime = WasmRuntime::new().unwrap();
    let plugin = runtime.load_plugin_without_capabilities("test_plugin.wasm").unwrap();
    
    let result = plugin.call_function("read_file", &["/etc/passwd"]);
    assert!(matches!(result, Err(PluginError::Permission(_))));
}

#[test]
fn test_authorized_access_succeeds() {
    let mut runtime = WasmRuntime::new().unwrap();
    let plugin = runtime.load_plugin_with_manifest("fs_plugin.wasm").unwrap();
    
    let result = plugin.call_function("read_file", &["/tmp/test.txt"]);
    assert!(result.is_ok());
}
```

### **2.2 Security Sandbox**
**Priority**: High
**Estimated Time**: 3-4 days
**Dependencies**: 2.1

#### Requirements
- WASI with restricted imports
- Path traversal prevention
- Resource consumption monitoring
- Audit logging for capability usage

#### Acceptance Criteria
- [ ] Prevent path traversal attacks (../../../etc/passwd)
- [ ] Log all capability usage for security auditing
- [ ] Resource limits enforced (memory, CPU, file handles)
- [ ] Plugin cannot escape sandbox even with crafted input

---

## Task 3: Palette Extension API (Register Actions)

### **3.1 Action Registry System**
**Priority**: High
**Estimated Time**: 4-5 days
**Dependencies**: 2.1

#### Requirements
- Plugin-registerable actions in command palette
- Action metadata (name, description, icon, shortcuts)
- Dynamic action loading and unloading
- Action execution with plugin callbacks

#### Implementation Guidelines
```rust
// File: crates/plugins-host/src/actions.rs
#[derive(Debug, Clone)]
pub struct Action {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub shortcut: Option<String>,
    pub icon: Option<String>,
    pub plugin_id: String,
}

pub trait ActionRegistry {
    fn register_action(&mut self, action: Action) -> Result<(), ActionError>;
    fn unregister_action(&mut self, action_id: &str) -> Result<(), ActionError>;
    fn execute_action(&self, action_id: &str, context: ActionContext) -> Result<ActionResult, ActionError>;
    fn list_actions(&self) -> Vec<Action>;
    fn search_actions(&self, query: &str) -> Vec<Action>;
}

// File: crates/plugins-api/src/palette.rs
pub trait PaletteExtension {
    fn register_actions(&self) -> Vec<ActionDefinition>;
    fn execute_action(&self, action_id: &str, args: &[Value]) -> Result<ActionResult, Error>;
}
```

#### Acceptance Criteria
- [ ] Plugin can register action that appears in palette
- [ ] Action executes and returns result within 2 seconds
- [ ] Multiple plugins can register actions without conflicts
- [ ] Action unregistration works when plugin unloads
- [ ] Search finds registered actions by name/description

#### Test Requirements
```rust
#[test]
fn test_action_registration() {
    let mut registry = ActionRegistry::new();
    let action = Action {
        id: "test.hello".to_string(),
        name: "Say Hello".to_string(),
        description: "Shows a hello message".to_string(),
        category: "test".to_string(),
        shortcut: Some("Ctrl+H".to_string()),
        icon: None,
        plugin_id: "test_plugin".to_string(),
    };
    
    registry.register_action(action).unwrap();
    assert_eq!(registry.list_actions().len(), 1);
}
```

### **3.2 Command Palette Integration**
**Priority**: High
**Estimated Time**: 3-4 days
**Dependencies**: 3.1, Phase 2 Command Palette

#### Requirements
- Extend existing command palette with plugin actions
- Categorization and filtering
- Action preview and documentation
- Keyboard navigation and shortcuts

#### Acceptance Criteria
- [ ] Plugin actions appear in existing command palette
- [ ] Actions grouped by category/plugin
- [ ] Keyboard shortcuts work for registered actions
- [ ] Action preview shows description and parameters

---

## Task 4: AI Provider Abstraction (Trait + OpenAI Implementation)

### **4.1 AI Provider Trait Design**
**Priority**: High
**Estimated Time**: 3-4 days
**Dependencies**: None

#### Requirements
- Generic AI provider trait for multiple services
- Request/response standardization
- Error handling and rate limiting
- Streaming and batch request support

#### Implementation Guidelines
```rust
// File: crates/ai/src/provider.rs
#[async_trait]
pub trait AiProvider {
    async fn explain_command(&self, command: &str, output: &str, error: &str) -> Result<AiResponse, AiError>;
    async fn suggest_fix(&self, error_output: &str) -> Result<AiResponse, AiError>;
    async fn complete_command(&self, partial: &str, context: &CommandContext) -> Result<Vec<Completion>, AiError>;
    async fn analyze_output(&self, output: &str) -> Result<OutputAnalysis, AiError>;
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub content: String,
    pub confidence: f32,
    pub sources: Vec<String>,
    pub model: String,
    pub tokens_used: u32,
}

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub shell: String,
    pub working_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub recent_commands: Vec<String>,
}
```

#### Acceptance Criteria
- [ ] Multiple AI providers can implement the trait
- [ ] Provider selection configurable at runtime
- [ ] Graceful fallback when provider unavailable
- [ ] Request context excludes sensitive information
- [ ] Response includes confidence scores

### **4.2 OpenAI Provider Implementation**
**Priority**: High
**Estimated Time**: 4-5 days
**Dependencies**: 4.1

#### Requirements
- OpenAI API integration using async HTTP client
- Configuration for API key, model, and parameters
- Error handling for API failures and rate limits
- Context-aware prompting for terminal assistance

#### Implementation Guidelines
```rust
// File: crates/ai/src/providers/openai.rs
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    rate_limiter: RateLimiter,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Result<Self, AiError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
            
        Ok(Self {
            client,
            api_key: config.api_key,
            model: config.model.unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
            base_url: config.base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            rate_limiter: RateLimiter::new(config.rate_limit),
        })
    }
}
```

#### Acceptance Criteria
- [ ] "Explain command" returns helpful explanation in ≤ 2s
- [ ] API key loaded from secure configuration
- [ ] Rate limiting prevents API quota exhaustion
- [ ] Network errors handled gracefully
- [ ] Response parsing robust against API changes

#### Test Requirements
```rust
#[test]
async fn test_explain_command() {
    let provider = OpenAiProvider::new(test_config()).unwrap();
    let response = provider.explain_command(
        "ls -la",
        "total 24\ndrwxr-xr-x 3 user user 4096 Nov 20 10:30 .",
        ""
    ).await.unwrap();
    
    assert!(!response.content.is_empty());
    assert!(response.confidence > 0.5);
}
```

### **4.3 AI Integration Features**
**Priority**: Medium
**Estimated Time**: 3-4 days
**Dependencies**: 4.2

#### Requirements
- Command explanation (right-click context menu)
- Error analysis and fix suggestions
- Command completion and suggestions
- Output summarization for long results

#### Acceptance Criteria
- [ ] Right-click on command shows "Explain with AI" option
- [ ] Error output triggers automatic fix suggestions
- [ ] AI suggestions appear in command completion
- [ ] Long output can be summarized with AI

---

## Task 5: Secret Redaction (Regex + Heuristics)

### **5.1 Secret Detection System**
**Priority**: High (Security)
**Estimated Time**: 4-5 days
**Dependencies**: None

#### Requirements
- Pattern-based detection for common secret types
- Configurable redaction rules
- Real-time scanning of terminal output
- Integration with AI provider to prevent data leakage

#### Implementation Guidelines
```rust
// File: crates/ai/src/redaction.rs
pub struct SecretRedactor {
    patterns: Vec<RedactionPattern>,
    heuristics: Vec<Box<dyn SecretHeuristic>>,
    config: RedactionConfig,
}

#[derive(Debug, Clone)]
pub struct RedactionPattern {
    pub name: String,
    pub regex: regex::Regex,
    pub replacement: String,
    pub confidence: f32,
}

pub trait SecretHeuristic {
    fn detect(&self, text: &str) -> Vec<SecretMatch>;
    fn name(&self) -> &str;
}

// File: crates/ai/src/heuristics.rs
pub struct AwsKeyHeuristic;
pub struct GithubTokenHeuristic;
pub struct PrivateKeyHeuristic;
pub struct PasswordHeuristic;
```

#### Acceptance Criteria
- [ ] AWS access keys detected and redacted in tests
- [ ] GitHub tokens detected and redacted in tests
- [ ] Private keys (SSH, PEM) detected and redacted
- [ ] False positive rate < 5% on normal terminal output
- [ ] Redaction adds < 10ms latency to output processing

#### Test Requirements
```rust
#[test]
fn test_aws_key_redaction() {
    let redactor = SecretRedactor::new();
    let input = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
    let output = redactor.redact(input);
    assert!(output.contains("AWS_ACCESS_KEY_ID=[REDACTED]"));
    assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn test_normal_output_unchanged() {
    let redactor = SecretRedactor::new();
    let input = "Hello world\nthis is normal terminal output";
    let output = redactor.redact(input);
    assert_eq!(input, output);
}
```

### **5.2 AI Context Sanitization**
**Priority**: High (Security)
**Estimated Time**: 2-3 days
**Dependencies**: 5.1, 4.1

#### Requirements
- Sanitize AI requests before sending to external providers
- Remove environment variables and file paths
- Redact potentially sensitive command arguments
- Audit log of what data is sent to AI providers

#### Acceptance Criteria
- [ ] Environment variables stripped from AI context
- [ ] File paths outside workspace redacted
- [ ] Secrets detected in command args before AI request
- [ ] Audit log tracks all data sent to AI providers

---

## Task 6: Theming System (Light/Dark + Import)

### **6.1 Theme Engine**
**Priority**: Medium
**Estimated Time**: 4-5 days
**Dependencies**: Phase 2 Config System

#### Requirements
- Built-in light and dark themes
- Custom theme import from files
- Dynamic theme switching without restart
- Plugin-extensible theme properties

#### Implementation Guidelines
```rust
// File: crates/config/src/theme.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub description: String,
    pub colors: ColorScheme,
    pub typography: Typography,
    pub spacing: Spacing,
    pub animations: AnimationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,
    pub ansi: [Color; 16],
    pub bright: [Color; 16],
    pub ui: UiColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiColors {
    pub palette_background: Color,
    pub palette_border: Color,
    pub palette_text: Color,
    pub palette_selected: Color,
}
```

#### Acceptance Criteria
- [ ] Switch between light/dark themes instantly
- [ ] Custom theme loaded from JSON/TOML file
- [ ] Theme changes persist across application restart
- [ ] All UI components respect theme colors
- [ ] Theme validation prevents invalid color values

#### Test Requirements
```rust
#[test]
fn test_theme_loading() {
    let theme_data = r#"
        {
            "name": "Test Theme",
            "colors": {
                "background": "#1e1e1e",
                "foreground": "#d4d4d4"
            }
        }
    "#;
    
    let theme: Theme = serde_json::from_str(theme_data).unwrap();
    assert_eq!(theme.name, "Test Theme");
}

#[test]
fn test_theme_persistence() {
    let mut config = Config::new().unwrap();
    config.set_theme("dark").unwrap();
    config.save().unwrap();
    
    let loaded_config = Config::load().unwrap();
    assert_eq!(loaded_config.current_theme(), "dark");
}
```

### **6.2 Theme Import and Management**
**Priority**: Medium
**Estimated Time**: 2-3 days
**Dependencies**: 6.1

#### Requirements
- Import themes from VS Code, Terminal.app, iTerm2
- Theme marketplace/repository support (future)
- Theme preview before applying
- Export current theme to file

#### Acceptance Criteria
- [ ] Import VS Code theme files successfully
- [ ] Theme preview shows sample terminal output
- [ ] Export current theme as JSON
- [ ] Theme validation on import

---

## Phase 3 Integration Testing

### **Integration Test Requirements**

#### Multi-Plugin System Test
```rust
#[test]
async fn test_multiple_plugins_with_ai() {
    let mut runtime = WasmRuntime::new().unwrap();
    let ai_provider = OpenAiProvider::new(test_config()).unwrap();
    
    // Load multiple plugins
    let palette_plugin = runtime.load_plugin("palette_plugin.wasm").unwrap();
    let util_plugin = runtime.load_plugin("util_plugin.wasm").unwrap();
    
    // Register actions from both plugins
    let mut registry = ActionRegistry::new();
    palette_plugin.register_actions(&mut registry).unwrap();
    util_plugin.register_actions(&mut registry).unwrap();
    
    // Test AI integration with plugin context
    let response = ai_provider.explain_command("custom_command", "", "").await.unwrap();
    assert!(!response.content.is_empty());
}
```

#### End-to-End Plugin Workflow
```rust
#[test]
fn test_plugin_lifecycle() {
    let mut host = PluginHost::new().unwrap();
    
    // 1. Load plugin with capabilities
    let plugin_id = host.load_plugin("test_plugin.wasm").unwrap();
    
    // 2. Register palette actions
    let actions = host.get_plugin_actions(&plugin_id).unwrap();
    assert!(!actions.is_empty());
    
    // 3. Execute action
    let result = host.execute_action(&actions[0].id, &[]).unwrap();
    assert!(result.success);
    
    // 4. Unload plugin
    host.unload_plugin(&plugin_id).unwrap();
    assert!(host.get_plugin_actions(&plugin_id).is_err());
}
```

---

## Performance Requirements

| Component | Metric | Target |
|-----------|--------|--------|
| Plugin Loading | Time to load | < 500ms |
| Action Execution | Plugin action response | < 2s |
| AI Explanation | Response time | < 2s (network permitting) |
| Secret Redaction | Processing latency | < 10ms |
| Theme Switching | UI update time | < 100ms |
| Memory Usage | Additional overhead | < 50MB per loaded plugin |

---

## Security Requirements

1. **Plugin Isolation**: Plugins cannot access host filesystem without explicit capability
2. **API Key Security**: AI provider keys stored in secure configuration, never logged
3. **Secret Redaction**: All terminal output scanned before AI requests
4. **Capability Enforcement**: Permission checks on every plugin API call
5. **Audit Trail**: All plugin actions and AI requests logged for security review

---

## Documentation Requirements

Each task must include:
1. API documentation with examples
2. Plugin development guide updates
3. Security best practices
4. Troubleshooting guide
5. Migration guide from Phase 2

---

## Validation and Testing

- Unit tests for all new APIs (>90% coverage)
- Integration tests for plugin interactions
- Security tests for capability violations
- Performance benchmarks for all operations
- Cross-platform testing (Linux, macOS, Windows)
- Example plugins demonstrating each capability

This comprehensive task breakdown provides AI coding agents with clear implementation targets, acceptance criteria, and testing requirements for successful Phase 3 completion.