const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["assets/DashboardPage.CNMptHoX.js","assets/react-vendor.i0I-mtkZ.js","assets/chart-vendor.VQ2MiRkI.js","assets/LoadingStates.-pyytTrq.js","assets/ui-vendor.B7raoZKG.js","assets/ServicesPage.CmiICeQ4.js","assets/LogsPage.CJxt3GcC.js","assets/ResponseTraceModal.BcxhNUIz.js","assets/sparkles.BIHDU0Rt.js","assets/file-code.CuI9rqEu.js","assets/MetricsPage.Cw2KF7jM.js","assets/VerificationPage.C-JPS3kd.js","assets/ContractDiffPage.Bl79VmwU.js","assets/AIStudioNav.D_Rc_Ra0.js","assets/IncidentDashboardPage.BTB6c-IS.js","assets/FitnessFunctionsPage.DI4hfpBK.js","assets/tag.DvzF3Eyb.js","assets/folder.D5ZfZl3g.js","assets/FixturesPage.B-8YdD7N.js","assets/TestingPage.De-Fcur8.js","assets/ImportPage.DpPkHDuE.js","assets/WorkspacesPage.CA4oueEK.js","assets/PlaygroundPage.GjwNBDPC.js","assets/PluginsPage.pREEuALG.js","assets/ChainsPage.Cr8o0PYP.js","assets/GraphPage.XVoJOfyW.js","assets/index.CE7kPTm3.js","assets/value.CVjnIB-I.js","assets/message-square.BL5RFAEx.js","assets/graphLayouts.DrlVMor4.js","assets/WorldStatePage.B4Uz0fNH.js","assets/useWebSocket.BQUZw17Q.js","assets/PerformancePage.QCDrGEz6.js","assets/square.Ba-FUup_.js","assets/ScenarioStateMachineEditor.c_q_v59Z.js","assets/useHistory.B447BaZ5.js","assets/ScenarioStudioPage.5sALfbGz.js","assets/AnalyticsPage.B5ytAR1q.js","assets/PillarAnalyticsPage.2cpxnFW_.js","assets/HostedMocksPage.CGb2jbHx.js","assets/ObservabilityPage.Cothqdf2.js","assets/TracesPage.BFT8tZRL.js","assets/TestGeneratorPage.DqTtvLl4.js","assets/TestExecutionDashboard.B6F-0BL5.js","assets/IntegrationTestBuilder.D6vXxWj-.js","assets/ChaosPage.TYn4CA_q.js","assets/ResiliencePage.BbdnN0sI.js","assets/RecorderPage.3kwHtLUF.js","assets/BehavioralCloningPage.CGZR3RJK.js","assets/snowflake.Bh4TVWkL.js","assets/OrchestrationBuilder.D2VUNLtZ.js","assets/OrchestrationExecutionView.BMkFSF7c.js","assets/PluginRegistryPage.BChYShpO.js","assets/TemplateMarketplacePage.0JBAO_OF.js","assets/ShowcasePage.BNPMnQ99.js","assets/communityApi.A5_l6I-9.js","assets/LearningHubPage.DH98z8lH.js","assets/UserManagementPage.VW349Hw_.js","assets/user-plus.Du1coAhR.js","assets/MockAIPage.5hxwO1jM.js","assets/MockAIOpenApiGeneratorPage.Ch-B5TAz.js","assets/MockAIRulesPage.FJlsCT58.js","assets/VoicePage.BtpGkUJl.js","assets/AIStudioPage.BPIQUcfV.js","assets/tauri.UVgQf7G2.js"])))=>i.map(i=>d[i]);
var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
import { c as clsx, r as reactExports, j as jsxRuntimeExports, a as create, p as persist, b as React, d as reactDomExports, u as useQuery, e as useQueryClient, f as useMutation, g as ReactDOM, Q as QueryClient, h as clientExports, i as QueryClientProvider } from "./react-vendor.i0I-mtkZ.js";
import { S as Slot, R as Root, T as Thumb } from "./ui-vendor.B7raoZKG.js";
(function polyfill() {
  const relList = document.createElement("link").relList;
  if (relList && relList.supports && relList.supports("modulepreload")) {
    return;
  }
  for (const link of document.querySelectorAll('link[rel="modulepreload"]')) {
    processPreload(link);
  }
  new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type !== "childList") {
        continue;
      }
      for (const node of mutation.addedNodes) {
        if (node.tagName === "LINK" && node.rel === "modulepreload")
          processPreload(node);
      }
    }
  }).observe(document, { childList: true, subtree: true });
  function getFetchOpts(link) {
    const fetchOpts = {};
    if (link.integrity) fetchOpts.integrity = link.integrity;
    if (link.referrerPolicy) fetchOpts.referrerPolicy = link.referrerPolicy;
    if (link.crossOrigin === "use-credentials")
      fetchOpts.credentials = "include";
    else if (link.crossOrigin === "anonymous") fetchOpts.credentials = "omit";
    else fetchOpts.credentials = "same-origin";
    return fetchOpts;
  }
  function processPreload(link) {
    if (link.ep)
      return;
    link.ep = true;
    const fetchOpts = getFetchOpts(link);
    fetch(link.href, fetchOpts);
  }
})();
const LOG_LEVELS = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3
};
class Logger {
  constructor() {
    __publicField(this, "config");
    this.config = {
      minLevel: "warn",
      enableConsole: true
    };
  }
  shouldLog(level) {
    return LOG_LEVELS[level] >= LOG_LEVELS[this.config.minLevel];
  }
  formatMessage(level, message, context) {
    const timestamp = (/* @__PURE__ */ new Date()).toISOString();
    const contextStr = context ? ` ${JSON.stringify(context)}` : "";
    return `[${timestamp}] [${level.toUpperCase()}] ${message}${contextStr}`;
  }
  debug(message, context) {
    if (this.shouldLog("debug") && this.config.enableConsole) {
      console.debug(this.formatMessage("debug", message, context));
    }
  }
  info(message, context) {
    if (this.shouldLog("info") && this.config.enableConsole) {
      console.info(this.formatMessage("info", message, context));
    }
  }
  warn(message, context) {
    if (this.shouldLog("warn") && this.config.enableConsole) {
      console.warn(this.formatMessage("warn", message, context));
    }
  }
  error(message, error, context) {
    if (this.shouldLog("error") && this.config.enableConsole) {
      const errorContext = {
        ...context,
        error: error instanceof Error ? {
          message: error.message,
          stack: error.stack,
          name: error.name
        } : error
      };
      console.error(this.formatMessage("error", message, errorContext));
    }
  }
  configure(config) {
    this.config = { ...this.config, ...config };
  }
}
const logger = new Logger();
const scriptRel = "modulepreload";
const assetsURL = function(dep) {
  return "/" + dep;
};
const seen = {};
const __vitePreload = function preload(baseModule, deps, importerUrl) {
  let promise = Promise.resolve();
  if (deps && deps.length > 0) {
    let allSettled2 = function(promises) {
      return Promise.all(
        promises.map(
          (p) => Promise.resolve(p).then(
            (value) => ({ status: "fulfilled", value }),
            (reason) => ({ status: "rejected", reason })
          )
        )
      );
    };
    document.getElementsByTagName("link");
    const cspNonceMeta = document.querySelector(
      "meta[property=csp-nonce]"
    );
    const cspNonce = (cspNonceMeta == null ? void 0 : cspNonceMeta.nonce) || (cspNonceMeta == null ? void 0 : cspNonceMeta.getAttribute("nonce"));
    promise = allSettled2(
      deps.map((dep) => {
        dep = assetsURL(dep);
        if (dep in seen) return;
        seen[dep] = true;
        const isCss = dep.endsWith(".css");
        const cssSelector = isCss ? '[rel="stylesheet"]' : "";
        if (document.querySelector(`link[href="${dep}"]${cssSelector}`)) {
          return;
        }
        const link = document.createElement("link");
        link.rel = isCss ? "stylesheet" : scriptRel;
        if (!isCss) {
          link.as = "script";
        }
        link.crossOrigin = "";
        link.href = dep;
        if (cspNonce) {
          link.setAttribute("nonce", cspNonce);
        }
        document.head.appendChild(link);
        if (isCss) {
          return new Promise((res, rej) => {
            link.addEventListener("load", res);
            link.addEventListener(
              "error",
              () => rej(new Error(`Unable to preload CSS for ${dep}`))
            );
          });
        }
      })
    );
  }
  function handlePreloadError(err) {
    const e = new Event("vite:preloadError", {
      cancelable: true
    });
    e.payload = err;
    window.dispatchEvent(e);
    if (!e.defaultPrevented) {
      throw err;
    }
  }
  return promise.then((res) => {
    for (const item of res || []) {
      if (item.status !== "rejected") continue;
      handlePreloadError(item.reason);
    }
    return baseModule().catch(handlePreloadError);
  });
};
const concatArrays = (array1, array2) => {
  const combinedArray = new Array(array1.length + array2.length);
  for (let i = 0; i < array1.length; i++) {
    combinedArray[i] = array1[i];
  }
  for (let i = 0; i < array2.length; i++) {
    combinedArray[array1.length + i] = array2[i];
  }
  return combinedArray;
};
const createClassValidatorObject = (classGroupId, validator) => ({
  classGroupId,
  validator
});
const createClassPartObject = (nextPart = /* @__PURE__ */ new Map(), validators = null, classGroupId) => ({
  nextPart,
  validators,
  classGroupId
});
const CLASS_PART_SEPARATOR = "-";
const EMPTY_CONFLICTS = [];
const ARBITRARY_PROPERTY_PREFIX = "arbitrary..";
const createClassGroupUtils = (config) => {
  const classMap = createClassMap(config);
  const {
    conflictingClassGroups,
    conflictingClassGroupModifiers
  } = config;
  const getClassGroupId = (className) => {
    if (className.startsWith("[") && className.endsWith("]")) {
      return getGroupIdForArbitraryProperty(className);
    }
    const classParts = className.split(CLASS_PART_SEPARATOR);
    const startIndex = classParts[0] === "" && classParts.length > 1 ? 1 : 0;
    return getGroupRecursive(classParts, startIndex, classMap);
  };
  const getConflictingClassGroupIds = (classGroupId, hasPostfixModifier) => {
    if (hasPostfixModifier) {
      const modifierConflicts = conflictingClassGroupModifiers[classGroupId];
      const baseConflicts = conflictingClassGroups[classGroupId];
      if (modifierConflicts) {
        if (baseConflicts) {
          return concatArrays(baseConflicts, modifierConflicts);
        }
        return modifierConflicts;
      }
      return baseConflicts || EMPTY_CONFLICTS;
    }
    return conflictingClassGroups[classGroupId] || EMPTY_CONFLICTS;
  };
  return {
    getClassGroupId,
    getConflictingClassGroupIds
  };
};
const getGroupRecursive = (classParts, startIndex, classPartObject) => {
  const classPathsLength = classParts.length - startIndex;
  if (classPathsLength === 0) {
    return classPartObject.classGroupId;
  }
  const currentClassPart = classParts[startIndex];
  const nextClassPartObject = classPartObject.nextPart.get(currentClassPart);
  if (nextClassPartObject) {
    const result = getGroupRecursive(classParts, startIndex + 1, nextClassPartObject);
    if (result) return result;
  }
  const validators = classPartObject.validators;
  if (validators === null) {
    return void 0;
  }
  const classRest = startIndex === 0 ? classParts.join(CLASS_PART_SEPARATOR) : classParts.slice(startIndex).join(CLASS_PART_SEPARATOR);
  const validatorsLength = validators.length;
  for (let i = 0; i < validatorsLength; i++) {
    const validatorObj = validators[i];
    if (validatorObj.validator(classRest)) {
      return validatorObj.classGroupId;
    }
  }
  return void 0;
};
const getGroupIdForArbitraryProperty = (className) => className.slice(1, -1).indexOf(":") === -1 ? void 0 : (() => {
  const content = className.slice(1, -1);
  const colonIndex = content.indexOf(":");
  const property = content.slice(0, colonIndex);
  return property ? ARBITRARY_PROPERTY_PREFIX + property : void 0;
})();
const createClassMap = (config) => {
  const {
    theme,
    classGroups
  } = config;
  return processClassGroups(classGroups, theme);
};
const processClassGroups = (classGroups, theme) => {
  const classMap = createClassPartObject();
  for (const classGroupId in classGroups) {
    const group = classGroups[classGroupId];
    processClassesRecursively(group, classMap, classGroupId, theme);
  }
  return classMap;
};
const processClassesRecursively = (classGroup, classPartObject, classGroupId, theme) => {
  const len = classGroup.length;
  for (let i = 0; i < len; i++) {
    const classDefinition = classGroup[i];
    processClassDefinition(classDefinition, classPartObject, classGroupId, theme);
  }
};
const processClassDefinition = (classDefinition, classPartObject, classGroupId, theme) => {
  if (typeof classDefinition === "string") {
    processStringDefinition(classDefinition, classPartObject, classGroupId);
    return;
  }
  if (typeof classDefinition === "function") {
    processFunctionDefinition(classDefinition, classPartObject, classGroupId, theme);
    return;
  }
  processObjectDefinition(classDefinition, classPartObject, classGroupId, theme);
};
const processStringDefinition = (classDefinition, classPartObject, classGroupId) => {
  const classPartObjectToEdit = classDefinition === "" ? classPartObject : getPart(classPartObject, classDefinition);
  classPartObjectToEdit.classGroupId = classGroupId;
};
const processFunctionDefinition = (classDefinition, classPartObject, classGroupId, theme) => {
  if (isThemeGetter(classDefinition)) {
    processClassesRecursively(classDefinition(theme), classPartObject, classGroupId, theme);
    return;
  }
  if (classPartObject.validators === null) {
    classPartObject.validators = [];
  }
  classPartObject.validators.push(createClassValidatorObject(classGroupId, classDefinition));
};
const processObjectDefinition = (classDefinition, classPartObject, classGroupId, theme) => {
  const entries = Object.entries(classDefinition);
  const len = entries.length;
  for (let i = 0; i < len; i++) {
    const [key, value] = entries[i];
    processClassesRecursively(value, getPart(classPartObject, key), classGroupId, theme);
  }
};
const getPart = (classPartObject, path) => {
  let current = classPartObject;
  const parts = path.split(CLASS_PART_SEPARATOR);
  const len = parts.length;
  for (let i = 0; i < len; i++) {
    const part = parts[i];
    let next = current.nextPart.get(part);
    if (!next) {
      next = createClassPartObject();
      current.nextPart.set(part, next);
    }
    current = next;
  }
  return current;
};
const isThemeGetter = (func) => "isThemeGetter" in func && func.isThemeGetter === true;
const createLruCache = (maxCacheSize) => {
  if (maxCacheSize < 1) {
    return {
      get: () => void 0,
      set: () => {
      }
    };
  }
  let cacheSize = 0;
  let cache = /* @__PURE__ */ Object.create(null);
  let previousCache = /* @__PURE__ */ Object.create(null);
  const update = (key, value) => {
    cache[key] = value;
    cacheSize++;
    if (cacheSize > maxCacheSize) {
      cacheSize = 0;
      previousCache = cache;
      cache = /* @__PURE__ */ Object.create(null);
    }
  };
  return {
    get(key) {
      let value = cache[key];
      if (value !== void 0) {
        return value;
      }
      if ((value = previousCache[key]) !== void 0) {
        update(key, value);
        return value;
      }
    },
    set(key, value) {
      if (key in cache) {
        cache[key] = value;
      } else {
        update(key, value);
      }
    }
  };
};
const IMPORTANT_MODIFIER = "!";
const MODIFIER_SEPARATOR = ":";
const EMPTY_MODIFIERS = [];
const createResultObject = (modifiers, hasImportantModifier, baseClassName, maybePostfixModifierPosition, isExternal) => ({
  modifiers,
  hasImportantModifier,
  baseClassName,
  maybePostfixModifierPosition,
  isExternal
});
const createParseClassName = (config) => {
  const {
    prefix,
    experimentalParseClassName
  } = config;
  let parseClassName = (className) => {
    const modifiers = [];
    let bracketDepth = 0;
    let parenDepth = 0;
    let modifierStart = 0;
    let postfixModifierPosition;
    const len = className.length;
    for (let index = 0; index < len; index++) {
      const currentCharacter = className[index];
      if (bracketDepth === 0 && parenDepth === 0) {
        if (currentCharacter === MODIFIER_SEPARATOR) {
          modifiers.push(className.slice(modifierStart, index));
          modifierStart = index + 1;
          continue;
        }
        if (currentCharacter === "/") {
          postfixModifierPosition = index;
          continue;
        }
      }
      if (currentCharacter === "[") bracketDepth++;
      else if (currentCharacter === "]") bracketDepth--;
      else if (currentCharacter === "(") parenDepth++;
      else if (currentCharacter === ")") parenDepth--;
    }
    const baseClassNameWithImportantModifier = modifiers.length === 0 ? className : className.slice(modifierStart);
    let baseClassName = baseClassNameWithImportantModifier;
    let hasImportantModifier = false;
    if (baseClassNameWithImportantModifier.endsWith(IMPORTANT_MODIFIER)) {
      baseClassName = baseClassNameWithImportantModifier.slice(0, -1);
      hasImportantModifier = true;
    } else if (
      /**
       * In Tailwind CSS v3 the important modifier was at the start of the base class name. This is still supported for legacy reasons.
       * @see https://github.com/dcastil/tailwind-merge/issues/513#issuecomment-2614029864
       */
      baseClassNameWithImportantModifier.startsWith(IMPORTANT_MODIFIER)
    ) {
      baseClassName = baseClassNameWithImportantModifier.slice(1);
      hasImportantModifier = true;
    }
    const maybePostfixModifierPosition = postfixModifierPosition && postfixModifierPosition > modifierStart ? postfixModifierPosition - modifierStart : void 0;
    return createResultObject(modifiers, hasImportantModifier, baseClassName, maybePostfixModifierPosition);
  };
  if (prefix) {
    const fullPrefix = prefix + MODIFIER_SEPARATOR;
    const parseClassNameOriginal = parseClassName;
    parseClassName = (className) => className.startsWith(fullPrefix) ? parseClassNameOriginal(className.slice(fullPrefix.length)) : createResultObject(EMPTY_MODIFIERS, false, className, void 0, true);
  }
  if (experimentalParseClassName) {
    const parseClassNameOriginal = parseClassName;
    parseClassName = (className) => experimentalParseClassName({
      className,
      parseClassName: parseClassNameOriginal
    });
  }
  return parseClassName;
};
const createSortModifiers = (config) => {
  const modifierWeights = /* @__PURE__ */ new Map();
  config.orderSensitiveModifiers.forEach((mod, index) => {
    modifierWeights.set(mod, 1e6 + index);
  });
  return (modifiers) => {
    const result = [];
    let currentSegment = [];
    for (let i = 0; i < modifiers.length; i++) {
      const modifier = modifiers[i];
      const isArbitrary = modifier[0] === "[";
      const isOrderSensitive = modifierWeights.has(modifier);
      if (isArbitrary || isOrderSensitive) {
        if (currentSegment.length > 0) {
          currentSegment.sort();
          result.push(...currentSegment);
          currentSegment = [];
        }
        result.push(modifier);
      } else {
        currentSegment.push(modifier);
      }
    }
    if (currentSegment.length > 0) {
      currentSegment.sort();
      result.push(...currentSegment);
    }
    return result;
  };
};
const createConfigUtils = (config) => ({
  cache: createLruCache(config.cacheSize),
  parseClassName: createParseClassName(config),
  sortModifiers: createSortModifiers(config),
  ...createClassGroupUtils(config)
});
const SPLIT_CLASSES_REGEX = /\s+/;
const mergeClassList = (classList, configUtils) => {
  const {
    parseClassName,
    getClassGroupId,
    getConflictingClassGroupIds,
    sortModifiers
  } = configUtils;
  const classGroupsInConflict = [];
  const classNames = classList.trim().split(SPLIT_CLASSES_REGEX);
  let result = "";
  for (let index = classNames.length - 1; index >= 0; index -= 1) {
    const originalClassName = classNames[index];
    const {
      isExternal,
      modifiers,
      hasImportantModifier,
      baseClassName,
      maybePostfixModifierPosition
    } = parseClassName(originalClassName);
    if (isExternal) {
      result = originalClassName + (result.length > 0 ? " " + result : result);
      continue;
    }
    let hasPostfixModifier = !!maybePostfixModifierPosition;
    let classGroupId = getClassGroupId(hasPostfixModifier ? baseClassName.substring(0, maybePostfixModifierPosition) : baseClassName);
    if (!classGroupId) {
      if (!hasPostfixModifier) {
        result = originalClassName + (result.length > 0 ? " " + result : result);
        continue;
      }
      classGroupId = getClassGroupId(baseClassName);
      if (!classGroupId) {
        result = originalClassName + (result.length > 0 ? " " + result : result);
        continue;
      }
      hasPostfixModifier = false;
    }
    const variantModifier = modifiers.length === 0 ? "" : modifiers.length === 1 ? modifiers[0] : sortModifiers(modifiers).join(":");
    const modifierId = hasImportantModifier ? variantModifier + IMPORTANT_MODIFIER : variantModifier;
    const classId = modifierId + classGroupId;
    if (classGroupsInConflict.indexOf(classId) > -1) {
      continue;
    }
    classGroupsInConflict.push(classId);
    const conflictGroups = getConflictingClassGroupIds(classGroupId, hasPostfixModifier);
    for (let i = 0; i < conflictGroups.length; ++i) {
      const group = conflictGroups[i];
      classGroupsInConflict.push(modifierId + group);
    }
    result = originalClassName + (result.length > 0 ? " " + result : result);
  }
  return result;
};
const twJoin = (...classLists) => {
  let index = 0;
  let argument;
  let resolvedValue;
  let string = "";
  while (index < classLists.length) {
    if (argument = classLists[index++]) {
      if (resolvedValue = toValue(argument)) {
        string && (string += " ");
        string += resolvedValue;
      }
    }
  }
  return string;
};
const toValue = (mix) => {
  if (typeof mix === "string") {
    return mix;
  }
  let resolvedValue;
  let string = "";
  for (let k = 0; k < mix.length; k++) {
    if (mix[k]) {
      if (resolvedValue = toValue(mix[k])) {
        string && (string += " ");
        string += resolvedValue;
      }
    }
  }
  return string;
};
const createTailwindMerge = (createConfigFirst, ...createConfigRest) => {
  let configUtils;
  let cacheGet;
  let cacheSet;
  let functionToCall;
  const initTailwindMerge = (classList) => {
    const config = createConfigRest.reduce((previousConfig, createConfigCurrent) => createConfigCurrent(previousConfig), createConfigFirst());
    configUtils = createConfigUtils(config);
    cacheGet = configUtils.cache.get;
    cacheSet = configUtils.cache.set;
    functionToCall = tailwindMerge;
    return tailwindMerge(classList);
  };
  const tailwindMerge = (classList) => {
    const cachedResult = cacheGet(classList);
    if (cachedResult) {
      return cachedResult;
    }
    const result = mergeClassList(classList, configUtils);
    cacheSet(classList, result);
    return result;
  };
  functionToCall = initTailwindMerge;
  return (...args) => functionToCall(twJoin(...args));
};
const fallbackThemeArr = [];
const fromTheme = (key) => {
  const themeGetter = (theme) => theme[key] || fallbackThemeArr;
  themeGetter.isThemeGetter = true;
  return themeGetter;
};
const arbitraryValueRegex = /^\[(?:(\w[\w-]*):)?(.+)\]$/i;
const arbitraryVariableRegex = /^\((?:(\w[\w-]*):)?(.+)\)$/i;
const fractionRegex = /^\d+\/\d+$/;
const tshirtUnitRegex = /^(\d+(\.\d+)?)?(xs|sm|md|lg|xl)$/;
const lengthUnitRegex = /\d+(%|px|r?em|[sdl]?v([hwib]|min|max)|pt|pc|in|cm|mm|cap|ch|ex|r?lh|cq(w|h|i|b|min|max))|\b(calc|min|max|clamp)\(.+\)|^0$/;
const colorFunctionRegex = /^(rgba?|hsla?|hwb|(ok)?(lab|lch)|color-mix)\(.+\)$/;
const shadowRegex = /^(inset_)?-?((\d+)?\.?(\d+)[a-z]+|0)_-?((\d+)?\.?(\d+)[a-z]+|0)/;
const imageRegex = /^(url|image|image-set|cross-fade|element|(repeating-)?(linear|radial|conic)-gradient)\(.+\)$/;
const isFraction = (value) => fractionRegex.test(value);
const isNumber = (value) => !!value && !Number.isNaN(Number(value));
const isInteger = (value) => !!value && Number.isInteger(Number(value));
const isPercent = (value) => value.endsWith("%") && isNumber(value.slice(0, -1));
const isTshirtSize = (value) => tshirtUnitRegex.test(value);
const isAny = () => true;
const isLengthOnly = (value) => (
  // `colorFunctionRegex` check is necessary because color functions can have percentages in them which which would be incorrectly classified as lengths.
  // For example, `hsl(0 0% 0%)` would be classified as a length without this check.
  // I could also use lookbehind assertion in `lengthUnitRegex` but that isn't supported widely enough.
  lengthUnitRegex.test(value) && !colorFunctionRegex.test(value)
);
const isNever = () => false;
const isShadow = (value) => shadowRegex.test(value);
const isImage = (value) => imageRegex.test(value);
const isAnyNonArbitrary = (value) => !isArbitraryValue(value) && !isArbitraryVariable(value);
const isArbitrarySize = (value) => getIsArbitraryValue(value, isLabelSize, isNever);
const isArbitraryValue = (value) => arbitraryValueRegex.test(value);
const isArbitraryLength = (value) => getIsArbitraryValue(value, isLabelLength, isLengthOnly);
const isArbitraryNumber = (value) => getIsArbitraryValue(value, isLabelNumber, isNumber);
const isArbitraryPosition = (value) => getIsArbitraryValue(value, isLabelPosition, isNever);
const isArbitraryImage = (value) => getIsArbitraryValue(value, isLabelImage, isImage);
const isArbitraryShadow = (value) => getIsArbitraryValue(value, isLabelShadow, isShadow);
const isArbitraryVariable = (value) => arbitraryVariableRegex.test(value);
const isArbitraryVariableLength = (value) => getIsArbitraryVariable(value, isLabelLength);
const isArbitraryVariableFamilyName = (value) => getIsArbitraryVariable(value, isLabelFamilyName);
const isArbitraryVariablePosition = (value) => getIsArbitraryVariable(value, isLabelPosition);
const isArbitraryVariableSize = (value) => getIsArbitraryVariable(value, isLabelSize);
const isArbitraryVariableImage = (value) => getIsArbitraryVariable(value, isLabelImage);
const isArbitraryVariableShadow = (value) => getIsArbitraryVariable(value, isLabelShadow, true);
const getIsArbitraryValue = (value, testLabel, testValue) => {
  const result = arbitraryValueRegex.exec(value);
  if (result) {
    if (result[1]) {
      return testLabel(result[1]);
    }
    return testValue(result[2]);
  }
  return false;
};
const getIsArbitraryVariable = (value, testLabel, shouldMatchNoLabel = false) => {
  const result = arbitraryVariableRegex.exec(value);
  if (result) {
    if (result[1]) {
      return testLabel(result[1]);
    }
    return shouldMatchNoLabel;
  }
  return false;
};
const isLabelPosition = (label) => label === "position" || label === "percentage";
const isLabelImage = (label) => label === "image" || label === "url";
const isLabelSize = (label) => label === "length" || label === "size" || label === "bg-size";
const isLabelLength = (label) => label === "length";
const isLabelNumber = (label) => label === "number";
const isLabelFamilyName = (label) => label === "family-name";
const isLabelShadow = (label) => label === "shadow";
const getDefaultConfig = () => {
  const themeColor = fromTheme("color");
  const themeFont = fromTheme("font");
  const themeText = fromTheme("text");
  const themeFontWeight = fromTheme("font-weight");
  const themeTracking = fromTheme("tracking");
  const themeLeading = fromTheme("leading");
  const themeBreakpoint = fromTheme("breakpoint");
  const themeContainer = fromTheme("container");
  const themeSpacing = fromTheme("spacing");
  const themeRadius = fromTheme("radius");
  const themeShadow = fromTheme("shadow");
  const themeInsetShadow = fromTheme("inset-shadow");
  const themeTextShadow = fromTheme("text-shadow");
  const themeDropShadow = fromTheme("drop-shadow");
  const themeBlur = fromTheme("blur");
  const themePerspective = fromTheme("perspective");
  const themeAspect = fromTheme("aspect");
  const themeEase = fromTheme("ease");
  const themeAnimate = fromTheme("animate");
  const scaleBreak = () => ["auto", "avoid", "all", "avoid-page", "page", "left", "right", "column"];
  const scalePosition = () => [
    "center",
    "top",
    "bottom",
    "left",
    "right",
    "top-left",
    // Deprecated since Tailwind CSS v4.1.0, see https://github.com/tailwindlabs/tailwindcss/pull/17378
    "left-top",
    "top-right",
    // Deprecated since Tailwind CSS v4.1.0, see https://github.com/tailwindlabs/tailwindcss/pull/17378
    "right-top",
    "bottom-right",
    // Deprecated since Tailwind CSS v4.1.0, see https://github.com/tailwindlabs/tailwindcss/pull/17378
    "right-bottom",
    "bottom-left",
    // Deprecated since Tailwind CSS v4.1.0, see https://github.com/tailwindlabs/tailwindcss/pull/17378
    "left-bottom"
  ];
  const scalePositionWithArbitrary = () => [...scalePosition(), isArbitraryVariable, isArbitraryValue];
  const scaleOverflow = () => ["auto", "hidden", "clip", "visible", "scroll"];
  const scaleOverscroll = () => ["auto", "contain", "none"];
  const scaleUnambiguousSpacing = () => [isArbitraryVariable, isArbitraryValue, themeSpacing];
  const scaleInset = () => [isFraction, "full", "auto", ...scaleUnambiguousSpacing()];
  const scaleGridTemplateColsRows = () => [isInteger, "none", "subgrid", isArbitraryVariable, isArbitraryValue];
  const scaleGridColRowStartAndEnd = () => ["auto", {
    span: ["full", isInteger, isArbitraryVariable, isArbitraryValue]
  }, isInteger, isArbitraryVariable, isArbitraryValue];
  const scaleGridColRowStartOrEnd = () => [isInteger, "auto", isArbitraryVariable, isArbitraryValue];
  const scaleGridAutoColsRows = () => ["auto", "min", "max", "fr", isArbitraryVariable, isArbitraryValue];
  const scaleAlignPrimaryAxis = () => ["start", "end", "center", "between", "around", "evenly", "stretch", "baseline", "center-safe", "end-safe"];
  const scaleAlignSecondaryAxis = () => ["start", "end", "center", "stretch", "center-safe", "end-safe"];
  const scaleMargin = () => ["auto", ...scaleUnambiguousSpacing()];
  const scaleSizing = () => [isFraction, "auto", "full", "dvw", "dvh", "lvw", "lvh", "svw", "svh", "min", "max", "fit", ...scaleUnambiguousSpacing()];
  const scaleColor = () => [themeColor, isArbitraryVariable, isArbitraryValue];
  const scaleBgPosition = () => [...scalePosition(), isArbitraryVariablePosition, isArbitraryPosition, {
    position: [isArbitraryVariable, isArbitraryValue]
  }];
  const scaleBgRepeat = () => ["no-repeat", {
    repeat: ["", "x", "y", "space", "round"]
  }];
  const scaleBgSize = () => ["auto", "cover", "contain", isArbitraryVariableSize, isArbitrarySize, {
    size: [isArbitraryVariable, isArbitraryValue]
  }];
  const scaleGradientStopPosition = () => [isPercent, isArbitraryVariableLength, isArbitraryLength];
  const scaleRadius = () => [
    // Deprecated since Tailwind CSS v4.0.0
    "",
    "none",
    "full",
    themeRadius,
    isArbitraryVariable,
    isArbitraryValue
  ];
  const scaleBorderWidth = () => ["", isNumber, isArbitraryVariableLength, isArbitraryLength];
  const scaleLineStyle = () => ["solid", "dashed", "dotted", "double"];
  const scaleBlendMode = () => ["normal", "multiply", "screen", "overlay", "darken", "lighten", "color-dodge", "color-burn", "hard-light", "soft-light", "difference", "exclusion", "hue", "saturation", "color", "luminosity"];
  const scaleMaskImagePosition = () => [isNumber, isPercent, isArbitraryVariablePosition, isArbitraryPosition];
  const scaleBlur = () => [
    // Deprecated since Tailwind CSS v4.0.0
    "",
    "none",
    themeBlur,
    isArbitraryVariable,
    isArbitraryValue
  ];
  const scaleRotate = () => ["none", isNumber, isArbitraryVariable, isArbitraryValue];
  const scaleScale = () => ["none", isNumber, isArbitraryVariable, isArbitraryValue];
  const scaleSkew = () => [isNumber, isArbitraryVariable, isArbitraryValue];
  const scaleTranslate = () => [isFraction, "full", ...scaleUnambiguousSpacing()];
  return {
    cacheSize: 500,
    theme: {
      animate: ["spin", "ping", "pulse", "bounce"],
      aspect: ["video"],
      blur: [isTshirtSize],
      breakpoint: [isTshirtSize],
      color: [isAny],
      container: [isTshirtSize],
      "drop-shadow": [isTshirtSize],
      ease: ["in", "out", "in-out"],
      font: [isAnyNonArbitrary],
      "font-weight": ["thin", "extralight", "light", "normal", "medium", "semibold", "bold", "extrabold", "black"],
      "inset-shadow": [isTshirtSize],
      leading: ["none", "tight", "snug", "normal", "relaxed", "loose"],
      perspective: ["dramatic", "near", "normal", "midrange", "distant", "none"],
      radius: [isTshirtSize],
      shadow: [isTshirtSize],
      spacing: ["px", isNumber],
      text: [isTshirtSize],
      "text-shadow": [isTshirtSize],
      tracking: ["tighter", "tight", "normal", "wide", "wider", "widest"]
    },
    classGroups: {
      // --------------
      // --- Layout ---
      // --------------
      /**
       * Aspect Ratio
       * @see https://tailwindcss.com/docs/aspect-ratio
       */
      aspect: [{
        aspect: ["auto", "square", isFraction, isArbitraryValue, isArbitraryVariable, themeAspect]
      }],
      /**
       * Container
       * @see https://tailwindcss.com/docs/container
       * @deprecated since Tailwind CSS v4.0.0
       */
      container: ["container"],
      /**
       * Columns
       * @see https://tailwindcss.com/docs/columns
       */
      columns: [{
        columns: [isNumber, isArbitraryValue, isArbitraryVariable, themeContainer]
      }],
      /**
       * Break After
       * @see https://tailwindcss.com/docs/break-after
       */
      "break-after": [{
        "break-after": scaleBreak()
      }],
      /**
       * Break Before
       * @see https://tailwindcss.com/docs/break-before
       */
      "break-before": [{
        "break-before": scaleBreak()
      }],
      /**
       * Break Inside
       * @see https://tailwindcss.com/docs/break-inside
       */
      "break-inside": [{
        "break-inside": ["auto", "avoid", "avoid-page", "avoid-column"]
      }],
      /**
       * Box Decoration Break
       * @see https://tailwindcss.com/docs/box-decoration-break
       */
      "box-decoration": [{
        "box-decoration": ["slice", "clone"]
      }],
      /**
       * Box Sizing
       * @see https://tailwindcss.com/docs/box-sizing
       */
      box: [{
        box: ["border", "content"]
      }],
      /**
       * Display
       * @see https://tailwindcss.com/docs/display
       */
      display: ["block", "inline-block", "inline", "flex", "inline-flex", "table", "inline-table", "table-caption", "table-cell", "table-column", "table-column-group", "table-footer-group", "table-header-group", "table-row-group", "table-row", "flow-root", "grid", "inline-grid", "contents", "list-item", "hidden"],
      /**
       * Screen Reader Only
       * @see https://tailwindcss.com/docs/display#screen-reader-only
       */
      sr: ["sr-only", "not-sr-only"],
      /**
       * Floats
       * @see https://tailwindcss.com/docs/float
       */
      float: [{
        float: ["right", "left", "none", "start", "end"]
      }],
      /**
       * Clear
       * @see https://tailwindcss.com/docs/clear
       */
      clear: [{
        clear: ["left", "right", "both", "none", "start", "end"]
      }],
      /**
       * Isolation
       * @see https://tailwindcss.com/docs/isolation
       */
      isolation: ["isolate", "isolation-auto"],
      /**
       * Object Fit
       * @see https://tailwindcss.com/docs/object-fit
       */
      "object-fit": [{
        object: ["contain", "cover", "fill", "none", "scale-down"]
      }],
      /**
       * Object Position
       * @see https://tailwindcss.com/docs/object-position
       */
      "object-position": [{
        object: scalePositionWithArbitrary()
      }],
      /**
       * Overflow
       * @see https://tailwindcss.com/docs/overflow
       */
      overflow: [{
        overflow: scaleOverflow()
      }],
      /**
       * Overflow X
       * @see https://tailwindcss.com/docs/overflow
       */
      "overflow-x": [{
        "overflow-x": scaleOverflow()
      }],
      /**
       * Overflow Y
       * @see https://tailwindcss.com/docs/overflow
       */
      "overflow-y": [{
        "overflow-y": scaleOverflow()
      }],
      /**
       * Overscroll Behavior
       * @see https://tailwindcss.com/docs/overscroll-behavior
       */
      overscroll: [{
        overscroll: scaleOverscroll()
      }],
      /**
       * Overscroll Behavior X
       * @see https://tailwindcss.com/docs/overscroll-behavior
       */
      "overscroll-x": [{
        "overscroll-x": scaleOverscroll()
      }],
      /**
       * Overscroll Behavior Y
       * @see https://tailwindcss.com/docs/overscroll-behavior
       */
      "overscroll-y": [{
        "overscroll-y": scaleOverscroll()
      }],
      /**
       * Position
       * @see https://tailwindcss.com/docs/position
       */
      position: ["static", "fixed", "absolute", "relative", "sticky"],
      /**
       * Top / Right / Bottom / Left
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      inset: [{
        inset: scaleInset()
      }],
      /**
       * Right / Left
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      "inset-x": [{
        "inset-x": scaleInset()
      }],
      /**
       * Top / Bottom
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      "inset-y": [{
        "inset-y": scaleInset()
      }],
      /**
       * Start
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      start: [{
        start: scaleInset()
      }],
      /**
       * End
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      end: [{
        end: scaleInset()
      }],
      /**
       * Top
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      top: [{
        top: scaleInset()
      }],
      /**
       * Right
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      right: [{
        right: scaleInset()
      }],
      /**
       * Bottom
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      bottom: [{
        bottom: scaleInset()
      }],
      /**
       * Left
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      left: [{
        left: scaleInset()
      }],
      /**
       * Visibility
       * @see https://tailwindcss.com/docs/visibility
       */
      visibility: ["visible", "invisible", "collapse"],
      /**
       * Z-Index
       * @see https://tailwindcss.com/docs/z-index
       */
      z: [{
        z: [isInteger, "auto", isArbitraryVariable, isArbitraryValue]
      }],
      // ------------------------
      // --- Flexbox and Grid ---
      // ------------------------
      /**
       * Flex Basis
       * @see https://tailwindcss.com/docs/flex-basis
       */
      basis: [{
        basis: [isFraction, "full", "auto", themeContainer, ...scaleUnambiguousSpacing()]
      }],
      /**
       * Flex Direction
       * @see https://tailwindcss.com/docs/flex-direction
       */
      "flex-direction": [{
        flex: ["row", "row-reverse", "col", "col-reverse"]
      }],
      /**
       * Flex Wrap
       * @see https://tailwindcss.com/docs/flex-wrap
       */
      "flex-wrap": [{
        flex: ["nowrap", "wrap", "wrap-reverse"]
      }],
      /**
       * Flex
       * @see https://tailwindcss.com/docs/flex
       */
      flex: [{
        flex: [isNumber, isFraction, "auto", "initial", "none", isArbitraryValue]
      }],
      /**
       * Flex Grow
       * @see https://tailwindcss.com/docs/flex-grow
       */
      grow: [{
        grow: ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Flex Shrink
       * @see https://tailwindcss.com/docs/flex-shrink
       */
      shrink: [{
        shrink: ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Order
       * @see https://tailwindcss.com/docs/order
       */
      order: [{
        order: [isInteger, "first", "last", "none", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Grid Template Columns
       * @see https://tailwindcss.com/docs/grid-template-columns
       */
      "grid-cols": [{
        "grid-cols": scaleGridTemplateColsRows()
      }],
      /**
       * Grid Column Start / End
       * @see https://tailwindcss.com/docs/grid-column
       */
      "col-start-end": [{
        col: scaleGridColRowStartAndEnd()
      }],
      /**
       * Grid Column Start
       * @see https://tailwindcss.com/docs/grid-column
       */
      "col-start": [{
        "col-start": scaleGridColRowStartOrEnd()
      }],
      /**
       * Grid Column End
       * @see https://tailwindcss.com/docs/grid-column
       */
      "col-end": [{
        "col-end": scaleGridColRowStartOrEnd()
      }],
      /**
       * Grid Template Rows
       * @see https://tailwindcss.com/docs/grid-template-rows
       */
      "grid-rows": [{
        "grid-rows": scaleGridTemplateColsRows()
      }],
      /**
       * Grid Row Start / End
       * @see https://tailwindcss.com/docs/grid-row
       */
      "row-start-end": [{
        row: scaleGridColRowStartAndEnd()
      }],
      /**
       * Grid Row Start
       * @see https://tailwindcss.com/docs/grid-row
       */
      "row-start": [{
        "row-start": scaleGridColRowStartOrEnd()
      }],
      /**
       * Grid Row End
       * @see https://tailwindcss.com/docs/grid-row
       */
      "row-end": [{
        "row-end": scaleGridColRowStartOrEnd()
      }],
      /**
       * Grid Auto Flow
       * @see https://tailwindcss.com/docs/grid-auto-flow
       */
      "grid-flow": [{
        "grid-flow": ["row", "col", "dense", "row-dense", "col-dense"]
      }],
      /**
       * Grid Auto Columns
       * @see https://tailwindcss.com/docs/grid-auto-columns
       */
      "auto-cols": [{
        "auto-cols": scaleGridAutoColsRows()
      }],
      /**
       * Grid Auto Rows
       * @see https://tailwindcss.com/docs/grid-auto-rows
       */
      "auto-rows": [{
        "auto-rows": scaleGridAutoColsRows()
      }],
      /**
       * Gap
       * @see https://tailwindcss.com/docs/gap
       */
      gap: [{
        gap: scaleUnambiguousSpacing()
      }],
      /**
       * Gap X
       * @see https://tailwindcss.com/docs/gap
       */
      "gap-x": [{
        "gap-x": scaleUnambiguousSpacing()
      }],
      /**
       * Gap Y
       * @see https://tailwindcss.com/docs/gap
       */
      "gap-y": [{
        "gap-y": scaleUnambiguousSpacing()
      }],
      /**
       * Justify Content
       * @see https://tailwindcss.com/docs/justify-content
       */
      "justify-content": [{
        justify: [...scaleAlignPrimaryAxis(), "normal"]
      }],
      /**
       * Justify Items
       * @see https://tailwindcss.com/docs/justify-items
       */
      "justify-items": [{
        "justify-items": [...scaleAlignSecondaryAxis(), "normal"]
      }],
      /**
       * Justify Self
       * @see https://tailwindcss.com/docs/justify-self
       */
      "justify-self": [{
        "justify-self": ["auto", ...scaleAlignSecondaryAxis()]
      }],
      /**
       * Align Content
       * @see https://tailwindcss.com/docs/align-content
       */
      "align-content": [{
        content: ["normal", ...scaleAlignPrimaryAxis()]
      }],
      /**
       * Align Items
       * @see https://tailwindcss.com/docs/align-items
       */
      "align-items": [{
        items: [...scaleAlignSecondaryAxis(), {
          baseline: ["", "last"]
        }]
      }],
      /**
       * Align Self
       * @see https://tailwindcss.com/docs/align-self
       */
      "align-self": [{
        self: ["auto", ...scaleAlignSecondaryAxis(), {
          baseline: ["", "last"]
        }]
      }],
      /**
       * Place Content
       * @see https://tailwindcss.com/docs/place-content
       */
      "place-content": [{
        "place-content": scaleAlignPrimaryAxis()
      }],
      /**
       * Place Items
       * @see https://tailwindcss.com/docs/place-items
       */
      "place-items": [{
        "place-items": [...scaleAlignSecondaryAxis(), "baseline"]
      }],
      /**
       * Place Self
       * @see https://tailwindcss.com/docs/place-self
       */
      "place-self": [{
        "place-self": ["auto", ...scaleAlignSecondaryAxis()]
      }],
      // Spacing
      /**
       * Padding
       * @see https://tailwindcss.com/docs/padding
       */
      p: [{
        p: scaleUnambiguousSpacing()
      }],
      /**
       * Padding X
       * @see https://tailwindcss.com/docs/padding
       */
      px: [{
        px: scaleUnambiguousSpacing()
      }],
      /**
       * Padding Y
       * @see https://tailwindcss.com/docs/padding
       */
      py: [{
        py: scaleUnambiguousSpacing()
      }],
      /**
       * Padding Start
       * @see https://tailwindcss.com/docs/padding
       */
      ps: [{
        ps: scaleUnambiguousSpacing()
      }],
      /**
       * Padding End
       * @see https://tailwindcss.com/docs/padding
       */
      pe: [{
        pe: scaleUnambiguousSpacing()
      }],
      /**
       * Padding Top
       * @see https://tailwindcss.com/docs/padding
       */
      pt: [{
        pt: scaleUnambiguousSpacing()
      }],
      /**
       * Padding Right
       * @see https://tailwindcss.com/docs/padding
       */
      pr: [{
        pr: scaleUnambiguousSpacing()
      }],
      /**
       * Padding Bottom
       * @see https://tailwindcss.com/docs/padding
       */
      pb: [{
        pb: scaleUnambiguousSpacing()
      }],
      /**
       * Padding Left
       * @see https://tailwindcss.com/docs/padding
       */
      pl: [{
        pl: scaleUnambiguousSpacing()
      }],
      /**
       * Margin
       * @see https://tailwindcss.com/docs/margin
       */
      m: [{
        m: scaleMargin()
      }],
      /**
       * Margin X
       * @see https://tailwindcss.com/docs/margin
       */
      mx: [{
        mx: scaleMargin()
      }],
      /**
       * Margin Y
       * @see https://tailwindcss.com/docs/margin
       */
      my: [{
        my: scaleMargin()
      }],
      /**
       * Margin Start
       * @see https://tailwindcss.com/docs/margin
       */
      ms: [{
        ms: scaleMargin()
      }],
      /**
       * Margin End
       * @see https://tailwindcss.com/docs/margin
       */
      me: [{
        me: scaleMargin()
      }],
      /**
       * Margin Top
       * @see https://tailwindcss.com/docs/margin
       */
      mt: [{
        mt: scaleMargin()
      }],
      /**
       * Margin Right
       * @see https://tailwindcss.com/docs/margin
       */
      mr: [{
        mr: scaleMargin()
      }],
      /**
       * Margin Bottom
       * @see https://tailwindcss.com/docs/margin
       */
      mb: [{
        mb: scaleMargin()
      }],
      /**
       * Margin Left
       * @see https://tailwindcss.com/docs/margin
       */
      ml: [{
        ml: scaleMargin()
      }],
      /**
       * Space Between X
       * @see https://tailwindcss.com/docs/margin#adding-space-between-children
       */
      "space-x": [{
        "space-x": scaleUnambiguousSpacing()
      }],
      /**
       * Space Between X Reverse
       * @see https://tailwindcss.com/docs/margin#adding-space-between-children
       */
      "space-x-reverse": ["space-x-reverse"],
      /**
       * Space Between Y
       * @see https://tailwindcss.com/docs/margin#adding-space-between-children
       */
      "space-y": [{
        "space-y": scaleUnambiguousSpacing()
      }],
      /**
       * Space Between Y Reverse
       * @see https://tailwindcss.com/docs/margin#adding-space-between-children
       */
      "space-y-reverse": ["space-y-reverse"],
      // --------------
      // --- Sizing ---
      // --------------
      /**
       * Size
       * @see https://tailwindcss.com/docs/width#setting-both-width-and-height
       */
      size: [{
        size: scaleSizing()
      }],
      /**
       * Width
       * @see https://tailwindcss.com/docs/width
       */
      w: [{
        w: [themeContainer, "screen", ...scaleSizing()]
      }],
      /**
       * Min-Width
       * @see https://tailwindcss.com/docs/min-width
       */
      "min-w": [{
        "min-w": [
          themeContainer,
          "screen",
          /** Deprecated. @see https://github.com/tailwindlabs/tailwindcss.com/issues/2027#issuecomment-2620152757 */
          "none",
          ...scaleSizing()
        ]
      }],
      /**
       * Max-Width
       * @see https://tailwindcss.com/docs/max-width
       */
      "max-w": [{
        "max-w": [
          themeContainer,
          "screen",
          "none",
          /** Deprecated since Tailwind CSS v4.0.0. @see https://github.com/tailwindlabs/tailwindcss.com/issues/2027#issuecomment-2620152757 */
          "prose",
          /** Deprecated since Tailwind CSS v4.0.0. @see https://github.com/tailwindlabs/tailwindcss.com/issues/2027#issuecomment-2620152757 */
          {
            screen: [themeBreakpoint]
          },
          ...scaleSizing()
        ]
      }],
      /**
       * Height
       * @see https://tailwindcss.com/docs/height
       */
      h: [{
        h: ["screen", "lh", ...scaleSizing()]
      }],
      /**
       * Min-Height
       * @see https://tailwindcss.com/docs/min-height
       */
      "min-h": [{
        "min-h": ["screen", "lh", "none", ...scaleSizing()]
      }],
      /**
       * Max-Height
       * @see https://tailwindcss.com/docs/max-height
       */
      "max-h": [{
        "max-h": ["screen", "lh", ...scaleSizing()]
      }],
      // ------------------
      // --- Typography ---
      // ------------------
      /**
       * Font Size
       * @see https://tailwindcss.com/docs/font-size
       */
      "font-size": [{
        text: ["base", themeText, isArbitraryVariableLength, isArbitraryLength]
      }],
      /**
       * Font Smoothing
       * @see https://tailwindcss.com/docs/font-smoothing
       */
      "font-smoothing": ["antialiased", "subpixel-antialiased"],
      /**
       * Font Style
       * @see https://tailwindcss.com/docs/font-style
       */
      "font-style": ["italic", "not-italic"],
      /**
       * Font Weight
       * @see https://tailwindcss.com/docs/font-weight
       */
      "font-weight": [{
        font: [themeFontWeight, isArbitraryVariable, isArbitraryNumber]
      }],
      /**
       * Font Stretch
       * @see https://tailwindcss.com/docs/font-stretch
       */
      "font-stretch": [{
        "font-stretch": ["ultra-condensed", "extra-condensed", "condensed", "semi-condensed", "normal", "semi-expanded", "expanded", "extra-expanded", "ultra-expanded", isPercent, isArbitraryValue]
      }],
      /**
       * Font Family
       * @see https://tailwindcss.com/docs/font-family
       */
      "font-family": [{
        font: [isArbitraryVariableFamilyName, isArbitraryValue, themeFont]
      }],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      "fvn-normal": ["normal-nums"],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      "fvn-ordinal": ["ordinal"],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      "fvn-slashed-zero": ["slashed-zero"],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      "fvn-figure": ["lining-nums", "oldstyle-nums"],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      "fvn-spacing": ["proportional-nums", "tabular-nums"],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      "fvn-fraction": ["diagonal-fractions", "stacked-fractions"],
      /**
       * Letter Spacing
       * @see https://tailwindcss.com/docs/letter-spacing
       */
      tracking: [{
        tracking: [themeTracking, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Line Clamp
       * @see https://tailwindcss.com/docs/line-clamp
       */
      "line-clamp": [{
        "line-clamp": [isNumber, "none", isArbitraryVariable, isArbitraryNumber]
      }],
      /**
       * Line Height
       * @see https://tailwindcss.com/docs/line-height
       */
      leading: [{
        leading: [
          /** Deprecated since Tailwind CSS v4.0.0. @see https://github.com/tailwindlabs/tailwindcss.com/issues/2027#issuecomment-2620152757 */
          themeLeading,
          ...scaleUnambiguousSpacing()
        ]
      }],
      /**
       * List Style Image
       * @see https://tailwindcss.com/docs/list-style-image
       */
      "list-image": [{
        "list-image": ["none", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * List Style Position
       * @see https://tailwindcss.com/docs/list-style-position
       */
      "list-style-position": [{
        list: ["inside", "outside"]
      }],
      /**
       * List Style Type
       * @see https://tailwindcss.com/docs/list-style-type
       */
      "list-style-type": [{
        list: ["disc", "decimal", "none", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Text Alignment
       * @see https://tailwindcss.com/docs/text-align
       */
      "text-alignment": [{
        text: ["left", "center", "right", "justify", "start", "end"]
      }],
      /**
       * Placeholder Color
       * @deprecated since Tailwind CSS v3.0.0
       * @see https://v3.tailwindcss.com/docs/placeholder-color
       */
      "placeholder-color": [{
        placeholder: scaleColor()
      }],
      /**
       * Text Color
       * @see https://tailwindcss.com/docs/text-color
       */
      "text-color": [{
        text: scaleColor()
      }],
      /**
       * Text Decoration
       * @see https://tailwindcss.com/docs/text-decoration
       */
      "text-decoration": ["underline", "overline", "line-through", "no-underline"],
      /**
       * Text Decoration Style
       * @see https://tailwindcss.com/docs/text-decoration-style
       */
      "text-decoration-style": [{
        decoration: [...scaleLineStyle(), "wavy"]
      }],
      /**
       * Text Decoration Thickness
       * @see https://tailwindcss.com/docs/text-decoration-thickness
       */
      "text-decoration-thickness": [{
        decoration: [isNumber, "from-font", "auto", isArbitraryVariable, isArbitraryLength]
      }],
      /**
       * Text Decoration Color
       * @see https://tailwindcss.com/docs/text-decoration-color
       */
      "text-decoration-color": [{
        decoration: scaleColor()
      }],
      /**
       * Text Underline Offset
       * @see https://tailwindcss.com/docs/text-underline-offset
       */
      "underline-offset": [{
        "underline-offset": [isNumber, "auto", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Text Transform
       * @see https://tailwindcss.com/docs/text-transform
       */
      "text-transform": ["uppercase", "lowercase", "capitalize", "normal-case"],
      /**
       * Text Overflow
       * @see https://tailwindcss.com/docs/text-overflow
       */
      "text-overflow": ["truncate", "text-ellipsis", "text-clip"],
      /**
       * Text Wrap
       * @see https://tailwindcss.com/docs/text-wrap
       */
      "text-wrap": [{
        text: ["wrap", "nowrap", "balance", "pretty"]
      }],
      /**
       * Text Indent
       * @see https://tailwindcss.com/docs/text-indent
       */
      indent: [{
        indent: scaleUnambiguousSpacing()
      }],
      /**
       * Vertical Alignment
       * @see https://tailwindcss.com/docs/vertical-align
       */
      "vertical-align": [{
        align: ["baseline", "top", "middle", "bottom", "text-top", "text-bottom", "sub", "super", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Whitespace
       * @see https://tailwindcss.com/docs/whitespace
       */
      whitespace: [{
        whitespace: ["normal", "nowrap", "pre", "pre-line", "pre-wrap", "break-spaces"]
      }],
      /**
       * Word Break
       * @see https://tailwindcss.com/docs/word-break
       */
      break: [{
        break: ["normal", "words", "all", "keep"]
      }],
      /**
       * Overflow Wrap
       * @see https://tailwindcss.com/docs/overflow-wrap
       */
      wrap: [{
        wrap: ["break-word", "anywhere", "normal"]
      }],
      /**
       * Hyphens
       * @see https://tailwindcss.com/docs/hyphens
       */
      hyphens: [{
        hyphens: ["none", "manual", "auto"]
      }],
      /**
       * Content
       * @see https://tailwindcss.com/docs/content
       */
      content: [{
        content: ["none", isArbitraryVariable, isArbitraryValue]
      }],
      // -------------------
      // --- Backgrounds ---
      // -------------------
      /**
       * Background Attachment
       * @see https://tailwindcss.com/docs/background-attachment
       */
      "bg-attachment": [{
        bg: ["fixed", "local", "scroll"]
      }],
      /**
       * Background Clip
       * @see https://tailwindcss.com/docs/background-clip
       */
      "bg-clip": [{
        "bg-clip": ["border", "padding", "content", "text"]
      }],
      /**
       * Background Origin
       * @see https://tailwindcss.com/docs/background-origin
       */
      "bg-origin": [{
        "bg-origin": ["border", "padding", "content"]
      }],
      /**
       * Background Position
       * @see https://tailwindcss.com/docs/background-position
       */
      "bg-position": [{
        bg: scaleBgPosition()
      }],
      /**
       * Background Repeat
       * @see https://tailwindcss.com/docs/background-repeat
       */
      "bg-repeat": [{
        bg: scaleBgRepeat()
      }],
      /**
       * Background Size
       * @see https://tailwindcss.com/docs/background-size
       */
      "bg-size": [{
        bg: scaleBgSize()
      }],
      /**
       * Background Image
       * @see https://tailwindcss.com/docs/background-image
       */
      "bg-image": [{
        bg: ["none", {
          linear: [{
            to: ["t", "tr", "r", "br", "b", "bl", "l", "tl"]
          }, isInteger, isArbitraryVariable, isArbitraryValue],
          radial: ["", isArbitraryVariable, isArbitraryValue],
          conic: [isInteger, isArbitraryVariable, isArbitraryValue]
        }, isArbitraryVariableImage, isArbitraryImage]
      }],
      /**
       * Background Color
       * @see https://tailwindcss.com/docs/background-color
       */
      "bg-color": [{
        bg: scaleColor()
      }],
      /**
       * Gradient Color Stops From Position
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      "gradient-from-pos": [{
        from: scaleGradientStopPosition()
      }],
      /**
       * Gradient Color Stops Via Position
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      "gradient-via-pos": [{
        via: scaleGradientStopPosition()
      }],
      /**
       * Gradient Color Stops To Position
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      "gradient-to-pos": [{
        to: scaleGradientStopPosition()
      }],
      /**
       * Gradient Color Stops From
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      "gradient-from": [{
        from: scaleColor()
      }],
      /**
       * Gradient Color Stops Via
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      "gradient-via": [{
        via: scaleColor()
      }],
      /**
       * Gradient Color Stops To
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      "gradient-to": [{
        to: scaleColor()
      }],
      // ---------------
      // --- Borders ---
      // ---------------
      /**
       * Border Radius
       * @see https://tailwindcss.com/docs/border-radius
       */
      rounded: [{
        rounded: scaleRadius()
      }],
      /**
       * Border Radius Start
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-s": [{
        "rounded-s": scaleRadius()
      }],
      /**
       * Border Radius End
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-e": [{
        "rounded-e": scaleRadius()
      }],
      /**
       * Border Radius Top
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-t": [{
        "rounded-t": scaleRadius()
      }],
      /**
       * Border Radius Right
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-r": [{
        "rounded-r": scaleRadius()
      }],
      /**
       * Border Radius Bottom
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-b": [{
        "rounded-b": scaleRadius()
      }],
      /**
       * Border Radius Left
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-l": [{
        "rounded-l": scaleRadius()
      }],
      /**
       * Border Radius Start Start
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-ss": [{
        "rounded-ss": scaleRadius()
      }],
      /**
       * Border Radius Start End
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-se": [{
        "rounded-se": scaleRadius()
      }],
      /**
       * Border Radius End End
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-ee": [{
        "rounded-ee": scaleRadius()
      }],
      /**
       * Border Radius End Start
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-es": [{
        "rounded-es": scaleRadius()
      }],
      /**
       * Border Radius Top Left
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-tl": [{
        "rounded-tl": scaleRadius()
      }],
      /**
       * Border Radius Top Right
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-tr": [{
        "rounded-tr": scaleRadius()
      }],
      /**
       * Border Radius Bottom Right
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-br": [{
        "rounded-br": scaleRadius()
      }],
      /**
       * Border Radius Bottom Left
       * @see https://tailwindcss.com/docs/border-radius
       */
      "rounded-bl": [{
        "rounded-bl": scaleRadius()
      }],
      /**
       * Border Width
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w": [{
        border: scaleBorderWidth()
      }],
      /**
       * Border Width X
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-x": [{
        "border-x": scaleBorderWidth()
      }],
      /**
       * Border Width Y
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-y": [{
        "border-y": scaleBorderWidth()
      }],
      /**
       * Border Width Start
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-s": [{
        "border-s": scaleBorderWidth()
      }],
      /**
       * Border Width End
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-e": [{
        "border-e": scaleBorderWidth()
      }],
      /**
       * Border Width Top
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-t": [{
        "border-t": scaleBorderWidth()
      }],
      /**
       * Border Width Right
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-r": [{
        "border-r": scaleBorderWidth()
      }],
      /**
       * Border Width Bottom
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-b": [{
        "border-b": scaleBorderWidth()
      }],
      /**
       * Border Width Left
       * @see https://tailwindcss.com/docs/border-width
       */
      "border-w-l": [{
        "border-l": scaleBorderWidth()
      }],
      /**
       * Divide Width X
       * @see https://tailwindcss.com/docs/border-width#between-children
       */
      "divide-x": [{
        "divide-x": scaleBorderWidth()
      }],
      /**
       * Divide Width X Reverse
       * @see https://tailwindcss.com/docs/border-width#between-children
       */
      "divide-x-reverse": ["divide-x-reverse"],
      /**
       * Divide Width Y
       * @see https://tailwindcss.com/docs/border-width#between-children
       */
      "divide-y": [{
        "divide-y": scaleBorderWidth()
      }],
      /**
       * Divide Width Y Reverse
       * @see https://tailwindcss.com/docs/border-width#between-children
       */
      "divide-y-reverse": ["divide-y-reverse"],
      /**
       * Border Style
       * @see https://tailwindcss.com/docs/border-style
       */
      "border-style": [{
        border: [...scaleLineStyle(), "hidden", "none"]
      }],
      /**
       * Divide Style
       * @see https://tailwindcss.com/docs/border-style#setting-the-divider-style
       */
      "divide-style": [{
        divide: [...scaleLineStyle(), "hidden", "none"]
      }],
      /**
       * Border Color
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color": [{
        border: scaleColor()
      }],
      /**
       * Border Color X
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-x": [{
        "border-x": scaleColor()
      }],
      /**
       * Border Color Y
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-y": [{
        "border-y": scaleColor()
      }],
      /**
       * Border Color S
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-s": [{
        "border-s": scaleColor()
      }],
      /**
       * Border Color E
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-e": [{
        "border-e": scaleColor()
      }],
      /**
       * Border Color Top
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-t": [{
        "border-t": scaleColor()
      }],
      /**
       * Border Color Right
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-r": [{
        "border-r": scaleColor()
      }],
      /**
       * Border Color Bottom
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-b": [{
        "border-b": scaleColor()
      }],
      /**
       * Border Color Left
       * @see https://tailwindcss.com/docs/border-color
       */
      "border-color-l": [{
        "border-l": scaleColor()
      }],
      /**
       * Divide Color
       * @see https://tailwindcss.com/docs/divide-color
       */
      "divide-color": [{
        divide: scaleColor()
      }],
      /**
       * Outline Style
       * @see https://tailwindcss.com/docs/outline-style
       */
      "outline-style": [{
        outline: [...scaleLineStyle(), "none", "hidden"]
      }],
      /**
       * Outline Offset
       * @see https://tailwindcss.com/docs/outline-offset
       */
      "outline-offset": [{
        "outline-offset": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Outline Width
       * @see https://tailwindcss.com/docs/outline-width
       */
      "outline-w": [{
        outline: ["", isNumber, isArbitraryVariableLength, isArbitraryLength]
      }],
      /**
       * Outline Color
       * @see https://tailwindcss.com/docs/outline-color
       */
      "outline-color": [{
        outline: scaleColor()
      }],
      // ---------------
      // --- Effects ---
      // ---------------
      /**
       * Box Shadow
       * @see https://tailwindcss.com/docs/box-shadow
       */
      shadow: [{
        shadow: [
          // Deprecated since Tailwind CSS v4.0.0
          "",
          "none",
          themeShadow,
          isArbitraryVariableShadow,
          isArbitraryShadow
        ]
      }],
      /**
       * Box Shadow Color
       * @see https://tailwindcss.com/docs/box-shadow#setting-the-shadow-color
       */
      "shadow-color": [{
        shadow: scaleColor()
      }],
      /**
       * Inset Box Shadow
       * @see https://tailwindcss.com/docs/box-shadow#adding-an-inset-shadow
       */
      "inset-shadow": [{
        "inset-shadow": ["none", themeInsetShadow, isArbitraryVariableShadow, isArbitraryShadow]
      }],
      /**
       * Inset Box Shadow Color
       * @see https://tailwindcss.com/docs/box-shadow#setting-the-inset-shadow-color
       */
      "inset-shadow-color": [{
        "inset-shadow": scaleColor()
      }],
      /**
       * Ring Width
       * @see https://tailwindcss.com/docs/box-shadow#adding-a-ring
       */
      "ring-w": [{
        ring: scaleBorderWidth()
      }],
      /**
       * Ring Width Inset
       * @see https://v3.tailwindcss.com/docs/ring-width#inset-rings
       * @deprecated since Tailwind CSS v4.0.0
       * @see https://github.com/tailwindlabs/tailwindcss/blob/v4.0.0/packages/tailwindcss/src/utilities.ts#L4158
       */
      "ring-w-inset": ["ring-inset"],
      /**
       * Ring Color
       * @see https://tailwindcss.com/docs/box-shadow#setting-the-ring-color
       */
      "ring-color": [{
        ring: scaleColor()
      }],
      /**
       * Ring Offset Width
       * @see https://v3.tailwindcss.com/docs/ring-offset-width
       * @deprecated since Tailwind CSS v4.0.0
       * @see https://github.com/tailwindlabs/tailwindcss/blob/v4.0.0/packages/tailwindcss/src/utilities.ts#L4158
       */
      "ring-offset-w": [{
        "ring-offset": [isNumber, isArbitraryLength]
      }],
      /**
       * Ring Offset Color
       * @see https://v3.tailwindcss.com/docs/ring-offset-color
       * @deprecated since Tailwind CSS v4.0.0
       * @see https://github.com/tailwindlabs/tailwindcss/blob/v4.0.0/packages/tailwindcss/src/utilities.ts#L4158
       */
      "ring-offset-color": [{
        "ring-offset": scaleColor()
      }],
      /**
       * Inset Ring Width
       * @see https://tailwindcss.com/docs/box-shadow#adding-an-inset-ring
       */
      "inset-ring-w": [{
        "inset-ring": scaleBorderWidth()
      }],
      /**
       * Inset Ring Color
       * @see https://tailwindcss.com/docs/box-shadow#setting-the-inset-ring-color
       */
      "inset-ring-color": [{
        "inset-ring": scaleColor()
      }],
      /**
       * Text Shadow
       * @see https://tailwindcss.com/docs/text-shadow
       */
      "text-shadow": [{
        "text-shadow": ["none", themeTextShadow, isArbitraryVariableShadow, isArbitraryShadow]
      }],
      /**
       * Text Shadow Color
       * @see https://tailwindcss.com/docs/text-shadow#setting-the-shadow-color
       */
      "text-shadow-color": [{
        "text-shadow": scaleColor()
      }],
      /**
       * Opacity
       * @see https://tailwindcss.com/docs/opacity
       */
      opacity: [{
        opacity: [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Mix Blend Mode
       * @see https://tailwindcss.com/docs/mix-blend-mode
       */
      "mix-blend": [{
        "mix-blend": [...scaleBlendMode(), "plus-darker", "plus-lighter"]
      }],
      /**
       * Background Blend Mode
       * @see https://tailwindcss.com/docs/background-blend-mode
       */
      "bg-blend": [{
        "bg-blend": scaleBlendMode()
      }],
      /**
       * Mask Clip
       * @see https://tailwindcss.com/docs/mask-clip
       */
      "mask-clip": [{
        "mask-clip": ["border", "padding", "content", "fill", "stroke", "view"]
      }, "mask-no-clip"],
      /**
       * Mask Composite
       * @see https://tailwindcss.com/docs/mask-composite
       */
      "mask-composite": [{
        mask: ["add", "subtract", "intersect", "exclude"]
      }],
      /**
       * Mask Image
       * @see https://tailwindcss.com/docs/mask-image
       */
      "mask-image-linear-pos": [{
        "mask-linear": [isNumber]
      }],
      "mask-image-linear-from-pos": [{
        "mask-linear-from": scaleMaskImagePosition()
      }],
      "mask-image-linear-to-pos": [{
        "mask-linear-to": scaleMaskImagePosition()
      }],
      "mask-image-linear-from-color": [{
        "mask-linear-from": scaleColor()
      }],
      "mask-image-linear-to-color": [{
        "mask-linear-to": scaleColor()
      }],
      "mask-image-t-from-pos": [{
        "mask-t-from": scaleMaskImagePosition()
      }],
      "mask-image-t-to-pos": [{
        "mask-t-to": scaleMaskImagePosition()
      }],
      "mask-image-t-from-color": [{
        "mask-t-from": scaleColor()
      }],
      "mask-image-t-to-color": [{
        "mask-t-to": scaleColor()
      }],
      "mask-image-r-from-pos": [{
        "mask-r-from": scaleMaskImagePosition()
      }],
      "mask-image-r-to-pos": [{
        "mask-r-to": scaleMaskImagePosition()
      }],
      "mask-image-r-from-color": [{
        "mask-r-from": scaleColor()
      }],
      "mask-image-r-to-color": [{
        "mask-r-to": scaleColor()
      }],
      "mask-image-b-from-pos": [{
        "mask-b-from": scaleMaskImagePosition()
      }],
      "mask-image-b-to-pos": [{
        "mask-b-to": scaleMaskImagePosition()
      }],
      "mask-image-b-from-color": [{
        "mask-b-from": scaleColor()
      }],
      "mask-image-b-to-color": [{
        "mask-b-to": scaleColor()
      }],
      "mask-image-l-from-pos": [{
        "mask-l-from": scaleMaskImagePosition()
      }],
      "mask-image-l-to-pos": [{
        "mask-l-to": scaleMaskImagePosition()
      }],
      "mask-image-l-from-color": [{
        "mask-l-from": scaleColor()
      }],
      "mask-image-l-to-color": [{
        "mask-l-to": scaleColor()
      }],
      "mask-image-x-from-pos": [{
        "mask-x-from": scaleMaskImagePosition()
      }],
      "mask-image-x-to-pos": [{
        "mask-x-to": scaleMaskImagePosition()
      }],
      "mask-image-x-from-color": [{
        "mask-x-from": scaleColor()
      }],
      "mask-image-x-to-color": [{
        "mask-x-to": scaleColor()
      }],
      "mask-image-y-from-pos": [{
        "mask-y-from": scaleMaskImagePosition()
      }],
      "mask-image-y-to-pos": [{
        "mask-y-to": scaleMaskImagePosition()
      }],
      "mask-image-y-from-color": [{
        "mask-y-from": scaleColor()
      }],
      "mask-image-y-to-color": [{
        "mask-y-to": scaleColor()
      }],
      "mask-image-radial": [{
        "mask-radial": [isArbitraryVariable, isArbitraryValue]
      }],
      "mask-image-radial-from-pos": [{
        "mask-radial-from": scaleMaskImagePosition()
      }],
      "mask-image-radial-to-pos": [{
        "mask-radial-to": scaleMaskImagePosition()
      }],
      "mask-image-radial-from-color": [{
        "mask-radial-from": scaleColor()
      }],
      "mask-image-radial-to-color": [{
        "mask-radial-to": scaleColor()
      }],
      "mask-image-radial-shape": [{
        "mask-radial": ["circle", "ellipse"]
      }],
      "mask-image-radial-size": [{
        "mask-radial": [{
          closest: ["side", "corner"],
          farthest: ["side", "corner"]
        }]
      }],
      "mask-image-radial-pos": [{
        "mask-radial-at": scalePosition()
      }],
      "mask-image-conic-pos": [{
        "mask-conic": [isNumber]
      }],
      "mask-image-conic-from-pos": [{
        "mask-conic-from": scaleMaskImagePosition()
      }],
      "mask-image-conic-to-pos": [{
        "mask-conic-to": scaleMaskImagePosition()
      }],
      "mask-image-conic-from-color": [{
        "mask-conic-from": scaleColor()
      }],
      "mask-image-conic-to-color": [{
        "mask-conic-to": scaleColor()
      }],
      /**
       * Mask Mode
       * @see https://tailwindcss.com/docs/mask-mode
       */
      "mask-mode": [{
        mask: ["alpha", "luminance", "match"]
      }],
      /**
       * Mask Origin
       * @see https://tailwindcss.com/docs/mask-origin
       */
      "mask-origin": [{
        "mask-origin": ["border", "padding", "content", "fill", "stroke", "view"]
      }],
      /**
       * Mask Position
       * @see https://tailwindcss.com/docs/mask-position
       */
      "mask-position": [{
        mask: scaleBgPosition()
      }],
      /**
       * Mask Repeat
       * @see https://tailwindcss.com/docs/mask-repeat
       */
      "mask-repeat": [{
        mask: scaleBgRepeat()
      }],
      /**
       * Mask Size
       * @see https://tailwindcss.com/docs/mask-size
       */
      "mask-size": [{
        mask: scaleBgSize()
      }],
      /**
       * Mask Type
       * @see https://tailwindcss.com/docs/mask-type
       */
      "mask-type": [{
        "mask-type": ["alpha", "luminance"]
      }],
      /**
       * Mask Image
       * @see https://tailwindcss.com/docs/mask-image
       */
      "mask-image": [{
        mask: ["none", isArbitraryVariable, isArbitraryValue]
      }],
      // ---------------
      // --- Filters ---
      // ---------------
      /**
       * Filter
       * @see https://tailwindcss.com/docs/filter
       */
      filter: [{
        filter: [
          // Deprecated since Tailwind CSS v3.0.0
          "",
          "none",
          isArbitraryVariable,
          isArbitraryValue
        ]
      }],
      /**
       * Blur
       * @see https://tailwindcss.com/docs/blur
       */
      blur: [{
        blur: scaleBlur()
      }],
      /**
       * Brightness
       * @see https://tailwindcss.com/docs/brightness
       */
      brightness: [{
        brightness: [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Contrast
       * @see https://tailwindcss.com/docs/contrast
       */
      contrast: [{
        contrast: [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Drop Shadow
       * @see https://tailwindcss.com/docs/drop-shadow
       */
      "drop-shadow": [{
        "drop-shadow": [
          // Deprecated since Tailwind CSS v4.0.0
          "",
          "none",
          themeDropShadow,
          isArbitraryVariableShadow,
          isArbitraryShadow
        ]
      }],
      /**
       * Drop Shadow Color
       * @see https://tailwindcss.com/docs/filter-drop-shadow#setting-the-shadow-color
       */
      "drop-shadow-color": [{
        "drop-shadow": scaleColor()
      }],
      /**
       * Grayscale
       * @see https://tailwindcss.com/docs/grayscale
       */
      grayscale: [{
        grayscale: ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Hue Rotate
       * @see https://tailwindcss.com/docs/hue-rotate
       */
      "hue-rotate": [{
        "hue-rotate": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Invert
       * @see https://tailwindcss.com/docs/invert
       */
      invert: [{
        invert: ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Saturate
       * @see https://tailwindcss.com/docs/saturate
       */
      saturate: [{
        saturate: [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Sepia
       * @see https://tailwindcss.com/docs/sepia
       */
      sepia: [{
        sepia: ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Filter
       * @see https://tailwindcss.com/docs/backdrop-filter
       */
      "backdrop-filter": [{
        "backdrop-filter": [
          // Deprecated since Tailwind CSS v3.0.0
          "",
          "none",
          isArbitraryVariable,
          isArbitraryValue
        ]
      }],
      /**
       * Backdrop Blur
       * @see https://tailwindcss.com/docs/backdrop-blur
       */
      "backdrop-blur": [{
        "backdrop-blur": scaleBlur()
      }],
      /**
       * Backdrop Brightness
       * @see https://tailwindcss.com/docs/backdrop-brightness
       */
      "backdrop-brightness": [{
        "backdrop-brightness": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Contrast
       * @see https://tailwindcss.com/docs/backdrop-contrast
       */
      "backdrop-contrast": [{
        "backdrop-contrast": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Grayscale
       * @see https://tailwindcss.com/docs/backdrop-grayscale
       */
      "backdrop-grayscale": [{
        "backdrop-grayscale": ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Hue Rotate
       * @see https://tailwindcss.com/docs/backdrop-hue-rotate
       */
      "backdrop-hue-rotate": [{
        "backdrop-hue-rotate": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Invert
       * @see https://tailwindcss.com/docs/backdrop-invert
       */
      "backdrop-invert": [{
        "backdrop-invert": ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Opacity
       * @see https://tailwindcss.com/docs/backdrop-opacity
       */
      "backdrop-opacity": [{
        "backdrop-opacity": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Saturate
       * @see https://tailwindcss.com/docs/backdrop-saturate
       */
      "backdrop-saturate": [{
        "backdrop-saturate": [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Backdrop Sepia
       * @see https://tailwindcss.com/docs/backdrop-sepia
       */
      "backdrop-sepia": [{
        "backdrop-sepia": ["", isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      // --------------
      // --- Tables ---
      // --------------
      /**
       * Border Collapse
       * @see https://tailwindcss.com/docs/border-collapse
       */
      "border-collapse": [{
        border: ["collapse", "separate"]
      }],
      /**
       * Border Spacing
       * @see https://tailwindcss.com/docs/border-spacing
       */
      "border-spacing": [{
        "border-spacing": scaleUnambiguousSpacing()
      }],
      /**
       * Border Spacing X
       * @see https://tailwindcss.com/docs/border-spacing
       */
      "border-spacing-x": [{
        "border-spacing-x": scaleUnambiguousSpacing()
      }],
      /**
       * Border Spacing Y
       * @see https://tailwindcss.com/docs/border-spacing
       */
      "border-spacing-y": [{
        "border-spacing-y": scaleUnambiguousSpacing()
      }],
      /**
       * Table Layout
       * @see https://tailwindcss.com/docs/table-layout
       */
      "table-layout": [{
        table: ["auto", "fixed"]
      }],
      /**
       * Caption Side
       * @see https://tailwindcss.com/docs/caption-side
       */
      caption: [{
        caption: ["top", "bottom"]
      }],
      // ---------------------------------
      // --- Transitions and Animation ---
      // ---------------------------------
      /**
       * Transition Property
       * @see https://tailwindcss.com/docs/transition-property
       */
      transition: [{
        transition: ["", "all", "colors", "opacity", "shadow", "transform", "none", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Transition Behavior
       * @see https://tailwindcss.com/docs/transition-behavior
       */
      "transition-behavior": [{
        transition: ["normal", "discrete"]
      }],
      /**
       * Transition Duration
       * @see https://tailwindcss.com/docs/transition-duration
       */
      duration: [{
        duration: [isNumber, "initial", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Transition Timing Function
       * @see https://tailwindcss.com/docs/transition-timing-function
       */
      ease: [{
        ease: ["linear", "initial", themeEase, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Transition Delay
       * @see https://tailwindcss.com/docs/transition-delay
       */
      delay: [{
        delay: [isNumber, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Animation
       * @see https://tailwindcss.com/docs/animation
       */
      animate: [{
        animate: ["none", themeAnimate, isArbitraryVariable, isArbitraryValue]
      }],
      // ------------------
      // --- Transforms ---
      // ------------------
      /**
       * Backface Visibility
       * @see https://tailwindcss.com/docs/backface-visibility
       */
      backface: [{
        backface: ["hidden", "visible"]
      }],
      /**
       * Perspective
       * @see https://tailwindcss.com/docs/perspective
       */
      perspective: [{
        perspective: [themePerspective, isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Perspective Origin
       * @see https://tailwindcss.com/docs/perspective-origin
       */
      "perspective-origin": [{
        "perspective-origin": scalePositionWithArbitrary()
      }],
      /**
       * Rotate
       * @see https://tailwindcss.com/docs/rotate
       */
      rotate: [{
        rotate: scaleRotate()
      }],
      /**
       * Rotate X
       * @see https://tailwindcss.com/docs/rotate
       */
      "rotate-x": [{
        "rotate-x": scaleRotate()
      }],
      /**
       * Rotate Y
       * @see https://tailwindcss.com/docs/rotate
       */
      "rotate-y": [{
        "rotate-y": scaleRotate()
      }],
      /**
       * Rotate Z
       * @see https://tailwindcss.com/docs/rotate
       */
      "rotate-z": [{
        "rotate-z": scaleRotate()
      }],
      /**
       * Scale
       * @see https://tailwindcss.com/docs/scale
       */
      scale: [{
        scale: scaleScale()
      }],
      /**
       * Scale X
       * @see https://tailwindcss.com/docs/scale
       */
      "scale-x": [{
        "scale-x": scaleScale()
      }],
      /**
       * Scale Y
       * @see https://tailwindcss.com/docs/scale
       */
      "scale-y": [{
        "scale-y": scaleScale()
      }],
      /**
       * Scale Z
       * @see https://tailwindcss.com/docs/scale
       */
      "scale-z": [{
        "scale-z": scaleScale()
      }],
      /**
       * Scale 3D
       * @see https://tailwindcss.com/docs/scale
       */
      "scale-3d": ["scale-3d"],
      /**
       * Skew
       * @see https://tailwindcss.com/docs/skew
       */
      skew: [{
        skew: scaleSkew()
      }],
      /**
       * Skew X
       * @see https://tailwindcss.com/docs/skew
       */
      "skew-x": [{
        "skew-x": scaleSkew()
      }],
      /**
       * Skew Y
       * @see https://tailwindcss.com/docs/skew
       */
      "skew-y": [{
        "skew-y": scaleSkew()
      }],
      /**
       * Transform
       * @see https://tailwindcss.com/docs/transform
       */
      transform: [{
        transform: [isArbitraryVariable, isArbitraryValue, "", "none", "gpu", "cpu"]
      }],
      /**
       * Transform Origin
       * @see https://tailwindcss.com/docs/transform-origin
       */
      "transform-origin": [{
        origin: scalePositionWithArbitrary()
      }],
      /**
       * Transform Style
       * @see https://tailwindcss.com/docs/transform-style
       */
      "transform-style": [{
        transform: ["3d", "flat"]
      }],
      /**
       * Translate
       * @see https://tailwindcss.com/docs/translate
       */
      translate: [{
        translate: scaleTranslate()
      }],
      /**
       * Translate X
       * @see https://tailwindcss.com/docs/translate
       */
      "translate-x": [{
        "translate-x": scaleTranslate()
      }],
      /**
       * Translate Y
       * @see https://tailwindcss.com/docs/translate
       */
      "translate-y": [{
        "translate-y": scaleTranslate()
      }],
      /**
       * Translate Z
       * @see https://tailwindcss.com/docs/translate
       */
      "translate-z": [{
        "translate-z": scaleTranslate()
      }],
      /**
       * Translate None
       * @see https://tailwindcss.com/docs/translate
       */
      "translate-none": ["translate-none"],
      // ---------------------
      // --- Interactivity ---
      // ---------------------
      /**
       * Accent Color
       * @see https://tailwindcss.com/docs/accent-color
       */
      accent: [{
        accent: scaleColor()
      }],
      /**
       * Appearance
       * @see https://tailwindcss.com/docs/appearance
       */
      appearance: [{
        appearance: ["none", "auto"]
      }],
      /**
       * Caret Color
       * @see https://tailwindcss.com/docs/just-in-time-mode#caret-color-utilities
       */
      "caret-color": [{
        caret: scaleColor()
      }],
      /**
       * Color Scheme
       * @see https://tailwindcss.com/docs/color-scheme
       */
      "color-scheme": [{
        scheme: ["normal", "dark", "light", "light-dark", "only-dark", "only-light"]
      }],
      /**
       * Cursor
       * @see https://tailwindcss.com/docs/cursor
       */
      cursor: [{
        cursor: ["auto", "default", "pointer", "wait", "text", "move", "help", "not-allowed", "none", "context-menu", "progress", "cell", "crosshair", "vertical-text", "alias", "copy", "no-drop", "grab", "grabbing", "all-scroll", "col-resize", "row-resize", "n-resize", "e-resize", "s-resize", "w-resize", "ne-resize", "nw-resize", "se-resize", "sw-resize", "ew-resize", "ns-resize", "nesw-resize", "nwse-resize", "zoom-in", "zoom-out", isArbitraryVariable, isArbitraryValue]
      }],
      /**
       * Field Sizing
       * @see https://tailwindcss.com/docs/field-sizing
       */
      "field-sizing": [{
        "field-sizing": ["fixed", "content"]
      }],
      /**
       * Pointer Events
       * @see https://tailwindcss.com/docs/pointer-events
       */
      "pointer-events": [{
        "pointer-events": ["auto", "none"]
      }],
      /**
       * Resize
       * @see https://tailwindcss.com/docs/resize
       */
      resize: [{
        resize: ["none", "", "y", "x"]
      }],
      /**
       * Scroll Behavior
       * @see https://tailwindcss.com/docs/scroll-behavior
       */
      "scroll-behavior": [{
        scroll: ["auto", "smooth"]
      }],
      /**
       * Scroll Margin
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-m": [{
        "scroll-m": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin X
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-mx": [{
        "scroll-mx": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin Y
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-my": [{
        "scroll-my": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin Start
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-ms": [{
        "scroll-ms": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin End
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-me": [{
        "scroll-me": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin Top
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-mt": [{
        "scroll-mt": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin Right
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-mr": [{
        "scroll-mr": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin Bottom
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-mb": [{
        "scroll-mb": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Margin Left
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      "scroll-ml": [{
        "scroll-ml": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-p": [{
        "scroll-p": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding X
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-px": [{
        "scroll-px": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding Y
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-py": [{
        "scroll-py": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding Start
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-ps": [{
        "scroll-ps": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding End
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-pe": [{
        "scroll-pe": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding Top
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-pt": [{
        "scroll-pt": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding Right
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-pr": [{
        "scroll-pr": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding Bottom
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-pb": [{
        "scroll-pb": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Padding Left
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      "scroll-pl": [{
        "scroll-pl": scaleUnambiguousSpacing()
      }],
      /**
       * Scroll Snap Align
       * @see https://tailwindcss.com/docs/scroll-snap-align
       */
      "snap-align": [{
        snap: ["start", "end", "center", "align-none"]
      }],
      /**
       * Scroll Snap Stop
       * @see https://tailwindcss.com/docs/scroll-snap-stop
       */
      "snap-stop": [{
        snap: ["normal", "always"]
      }],
      /**
       * Scroll Snap Type
       * @see https://tailwindcss.com/docs/scroll-snap-type
       */
      "snap-type": [{
        snap: ["none", "x", "y", "both"]
      }],
      /**
       * Scroll Snap Type Strictness
       * @see https://tailwindcss.com/docs/scroll-snap-type
       */
      "snap-strictness": [{
        snap: ["mandatory", "proximity"]
      }],
      /**
       * Touch Action
       * @see https://tailwindcss.com/docs/touch-action
       */
      touch: [{
        touch: ["auto", "none", "manipulation"]
      }],
      /**
       * Touch Action X
       * @see https://tailwindcss.com/docs/touch-action
       */
      "touch-x": [{
        "touch-pan": ["x", "left", "right"]
      }],
      /**
       * Touch Action Y
       * @see https://tailwindcss.com/docs/touch-action
       */
      "touch-y": [{
        "touch-pan": ["y", "up", "down"]
      }],
      /**
       * Touch Action Pinch Zoom
       * @see https://tailwindcss.com/docs/touch-action
       */
      "touch-pz": ["touch-pinch-zoom"],
      /**
       * User Select
       * @see https://tailwindcss.com/docs/user-select
       */
      select: [{
        select: ["none", "text", "all", "auto"]
      }],
      /**
       * Will Change
       * @see https://tailwindcss.com/docs/will-change
       */
      "will-change": [{
        "will-change": ["auto", "scroll", "contents", "transform", isArbitraryVariable, isArbitraryValue]
      }],
      // -----------
      // --- SVG ---
      // -----------
      /**
       * Fill
       * @see https://tailwindcss.com/docs/fill
       */
      fill: [{
        fill: ["none", ...scaleColor()]
      }],
      /**
       * Stroke Width
       * @see https://tailwindcss.com/docs/stroke-width
       */
      "stroke-w": [{
        stroke: [isNumber, isArbitraryVariableLength, isArbitraryLength, isArbitraryNumber]
      }],
      /**
       * Stroke
       * @see https://tailwindcss.com/docs/stroke
       */
      stroke: [{
        stroke: ["none", ...scaleColor()]
      }],
      // ---------------------
      // --- Accessibility ---
      // ---------------------
      /**
       * Forced Color Adjust
       * @see https://tailwindcss.com/docs/forced-color-adjust
       */
      "forced-color-adjust": [{
        "forced-color-adjust": ["auto", "none"]
      }]
    },
    conflictingClassGroups: {
      overflow: ["overflow-x", "overflow-y"],
      overscroll: ["overscroll-x", "overscroll-y"],
      inset: ["inset-x", "inset-y", "start", "end", "top", "right", "bottom", "left"],
      "inset-x": ["right", "left"],
      "inset-y": ["top", "bottom"],
      flex: ["basis", "grow", "shrink"],
      gap: ["gap-x", "gap-y"],
      p: ["px", "py", "ps", "pe", "pt", "pr", "pb", "pl"],
      px: ["pr", "pl"],
      py: ["pt", "pb"],
      m: ["mx", "my", "ms", "me", "mt", "mr", "mb", "ml"],
      mx: ["mr", "ml"],
      my: ["mt", "mb"],
      size: ["w", "h"],
      "font-size": ["leading"],
      "fvn-normal": ["fvn-ordinal", "fvn-slashed-zero", "fvn-figure", "fvn-spacing", "fvn-fraction"],
      "fvn-ordinal": ["fvn-normal"],
      "fvn-slashed-zero": ["fvn-normal"],
      "fvn-figure": ["fvn-normal"],
      "fvn-spacing": ["fvn-normal"],
      "fvn-fraction": ["fvn-normal"],
      "line-clamp": ["display", "overflow"],
      rounded: ["rounded-s", "rounded-e", "rounded-t", "rounded-r", "rounded-b", "rounded-l", "rounded-ss", "rounded-se", "rounded-ee", "rounded-es", "rounded-tl", "rounded-tr", "rounded-br", "rounded-bl"],
      "rounded-s": ["rounded-ss", "rounded-es"],
      "rounded-e": ["rounded-se", "rounded-ee"],
      "rounded-t": ["rounded-tl", "rounded-tr"],
      "rounded-r": ["rounded-tr", "rounded-br"],
      "rounded-b": ["rounded-br", "rounded-bl"],
      "rounded-l": ["rounded-tl", "rounded-bl"],
      "border-spacing": ["border-spacing-x", "border-spacing-y"],
      "border-w": ["border-w-x", "border-w-y", "border-w-s", "border-w-e", "border-w-t", "border-w-r", "border-w-b", "border-w-l"],
      "border-w-x": ["border-w-r", "border-w-l"],
      "border-w-y": ["border-w-t", "border-w-b"],
      "border-color": ["border-color-x", "border-color-y", "border-color-s", "border-color-e", "border-color-t", "border-color-r", "border-color-b", "border-color-l"],
      "border-color-x": ["border-color-r", "border-color-l"],
      "border-color-y": ["border-color-t", "border-color-b"],
      translate: ["translate-x", "translate-y", "translate-none"],
      "translate-none": ["translate", "translate-x", "translate-y", "translate-z"],
      "scroll-m": ["scroll-mx", "scroll-my", "scroll-ms", "scroll-me", "scroll-mt", "scroll-mr", "scroll-mb", "scroll-ml"],
      "scroll-mx": ["scroll-mr", "scroll-ml"],
      "scroll-my": ["scroll-mt", "scroll-mb"],
      "scroll-p": ["scroll-px", "scroll-py", "scroll-ps", "scroll-pe", "scroll-pt", "scroll-pr", "scroll-pb", "scroll-pl"],
      "scroll-px": ["scroll-pr", "scroll-pl"],
      "scroll-py": ["scroll-pt", "scroll-pb"],
      touch: ["touch-x", "touch-y", "touch-pz"],
      "touch-x": ["touch"],
      "touch-y": ["touch"],
      "touch-pz": ["touch"]
    },
    conflictingClassGroupModifiers: {
      "font-size": ["leading"]
    },
    orderSensitiveModifiers: ["*", "**", "after", "backdrop", "before", "details-content", "file", "first-letter", "first-line", "marker", "placeholder", "selection"]
  };
};
const twMerge = /* @__PURE__ */ createTailwindMerge(getDefaultConfig);
function cn(...inputs) {
  return twMerge(clsx(inputs));
}
const falsyToString = (value) => typeof value === "boolean" ? `${value}` : value === 0 ? "0" : value;
const cx = clsx;
const cva = (base, config) => (props) => {
  var _config_compoundVariants;
  if ((config === null || config === void 0 ? void 0 : config.variants) == null) return cx(base, props === null || props === void 0 ? void 0 : props.class, props === null || props === void 0 ? void 0 : props.className);
  const { variants, defaultVariants } = config;
  const getVariantClassNames = Object.keys(variants).map((variant) => {
    const variantProp = props === null || props === void 0 ? void 0 : props[variant];
    const defaultVariantProp = defaultVariants === null || defaultVariants === void 0 ? void 0 : defaultVariants[variant];
    if (variantProp === null) return null;
    const variantKey = falsyToString(variantProp) || falsyToString(defaultVariantProp);
    return variants[variant][variantKey];
  });
  const propsWithoutUndefined = props && Object.entries(props).reduce((acc, param) => {
    let [key, value] = param;
    if (value === void 0) {
      return acc;
    }
    acc[key] = value;
    return acc;
  }, {});
  const getCompoundVariantClassNames = config === null || config === void 0 ? void 0 : (_config_compoundVariants = config.compoundVariants) === null || _config_compoundVariants === void 0 ? void 0 : _config_compoundVariants.reduce((acc, param) => {
    let { class: cvClass, className: cvClassName, ...compoundVariantOptions } = param;
    return Object.entries(compoundVariantOptions).every((param2) => {
      let [key, value] = param2;
      return Array.isArray(value) ? value.includes({
        ...defaultVariants,
        ...propsWithoutUndefined
      }[key]) : {
        ...defaultVariants,
        ...propsWithoutUndefined
      }[key] === value;
    }) ? [
      ...acc,
      cvClass,
      cvClassName
    ] : acc;
  }, []);
  return cx(base, getVariantClassNames, getCompoundVariantClassNames, props === null || props === void 0 ? void 0 : props.class, props === null || props === void 0 ? void 0 : props.className);
};
const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium text-foreground transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-sm hover:bg-primary/90 hover:shadow-md active:scale-[0.98]",
        destructive: "bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 hover:shadow-md active:scale-[0.98]",
        outline: "border border-input bg-background shadow-sm hover:bg-accent hover:text-accent-foreground hover:shadow-sm active:scale-[0.98]",
        secondary: "bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80 hover:shadow-md active:scale-[0.98]",
        ghost: "hover:bg-accent hover:text-accent-foreground hover:shadow-sm active:scale-[0.98]",
        link: "text-primary underline-offset-4 hover:underline"
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
        icon: "h-10 w-10"
      }
    },
    defaultVariants: {
      variant: "default",
      size: "default"
    }
  }
);
const Button$1 = reactExports.forwardRef(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return /* @__PURE__ */ jsxRuntimeExports.jsx(
      Comp,
      {
        className: cn(buttonVariants({ variant, size, className })),
        ref,
        ...props
      }
    );
  }
);
Button$1.displayName = "Button";
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const toKebabCase = (string) => string.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
const toCamelCase = (string) => string.replace(
  /^([A-Z])|[\s-_]+(\w)/g,
  (match, p1, p2) => p2 ? p2.toUpperCase() : p1.toLowerCase()
);
const toPascalCase = (string) => {
  const camelCase = toCamelCase(string);
  return camelCase.charAt(0).toUpperCase() + camelCase.slice(1);
};
const mergeClasses = (...classes) => classes.filter((className, index, array) => {
  return Boolean(className) && className.trim() !== "" && array.indexOf(className) === index;
}).join(" ").trim();
const hasA11yProp = (props) => {
  for (const prop in props) {
    if (prop.startsWith("aria-") || prop === "role" || prop === "title") {
      return true;
    }
  }
};
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
var defaultAttributes = {
  xmlns: "http://www.w3.org/2000/svg",
  width: 24,
  height: 24,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round",
  strokeLinejoin: "round"
};
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const Icon$1 = reactExports.forwardRef(
  ({
    color = "currentColor",
    size = 24,
    strokeWidth = 2,
    absoluteStrokeWidth,
    className = "",
    children,
    iconNode,
    ...rest
  }, ref) => reactExports.createElement(
    "svg",
    {
      ref,
      ...defaultAttributes,
      width: size,
      height: size,
      stroke: color,
      strokeWidth: absoluteStrokeWidth ? Number(strokeWidth) * 24 / Number(size) : strokeWidth,
      className: mergeClasses("lucide", className),
      ...!children && !hasA11yProp(rest) && { "aria-hidden": "true" },
      ...rest
    },
    [
      ...iconNode.map(([tag, attrs]) => reactExports.createElement(tag, attrs)),
      ...Array.isArray(children) ? children : [children]
    ]
  )
);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const createLucideIcon = (iconName, iconNode) => {
  const Component = reactExports.forwardRef(
    ({ className, ...props }, ref) => reactExports.createElement(Icon$1, {
      ref,
      iconNode,
      className: mergeClasses(
        `lucide-${toKebabCase(toPascalCase(iconName))}`,
        `lucide-${iconName}`,
        className
      ),
      ...props
    })
  );
  Component.displayName = toPascalCase(iconName);
  return Component;
};
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1B = [
  [
    "path",
    {
      d: "M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2",
      key: "169zse"
    }
  ]
];
const Activity = createLucideIcon("activity", __iconNode$1B);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1A = [
  ["path", { d: "M5 12h14", key: "1ays0h" }],
  ["path", { d: "m12 5 7 7-7 7", key: "xquz4c" }]
];
const ArrowRight = createLucideIcon("arrow-right", __iconNode$1A);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1z = [
  ["path", { d: "M10.268 21a2 2 0 0 0 3.464 0", key: "vwvbt9" }],
  [
    "path",
    {
      d: "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
      key: "11g9vi"
    }
  ]
];
const Bell = createLucideIcon("bell", __iconNode$1z);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1y = [
  ["path", { d: "M12 7v14", key: "1akyts" }],
  [
    "path",
    {
      d: "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",
      key: "ruj8y"
    }
  ]
];
const BookOpen = createLucideIcon("book-open", __iconNode$1y);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1x = [
  [
    "path",
    {
      d: "M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20",
      key: "k3hazp"
    }
  ]
];
const Book = createLucideIcon("book", __iconNode$1x);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1w = [
  ["path", { d: "M12 18V5", key: "adv99a" }],
  ["path", { d: "M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4", key: "1e3is1" }],
  ["path", { d: "M17.598 6.5A3 3 0 1 0 12 5a3 3 0 1 0-5.598 1.5", key: "1gqd8o" }],
  ["path", { d: "M17.997 5.125a4 4 0 0 1 2.526 5.77", key: "iwvgf7" }],
  ["path", { d: "M18 18a4 4 0 0 0 2-7.464", key: "efp6ie" }],
  ["path", { d: "M19.967 17.483A4 4 0 1 1 12 18a4 4 0 1 1-7.967-.517", key: "1gq6am" }],
  ["path", { d: "M6 18a4 4 0 0 1-2-7.464", key: "k1g0md" }],
  ["path", { d: "M6.003 5.125a4 4 0 0 0-2.526 5.77", key: "q97ue3" }]
];
const Brain = createLucideIcon("brain", __iconNode$1w);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1v = [
  ["path", { d: "M6 22V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v18Z", key: "1b4qmf" }],
  ["path", { d: "M6 12H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2", key: "i71pzd" }],
  ["path", { d: "M18 9h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-2", key: "10jefs" }],
  ["path", { d: "M10 6h4", key: "1itunk" }],
  ["path", { d: "M10 10h4", key: "tcdvrf" }],
  ["path", { d: "M10 14h4", key: "kelpxr" }],
  ["path", { d: "M10 18h4", key: "1ulq68" }]
];
const Building2 = createLucideIcon("building-2", __iconNode$1v);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1u = [
  ["path", { d: "M8 2v4", key: "1cmpym" }],
  ["path", { d: "M16 2v4", key: "4m81vk" }],
  ["rect", { width: "18", height: "18", x: "3", y: "4", rx: "2", key: "1hopcy" }],
  ["path", { d: "M3 10h18", key: "8toen8" }]
];
const Calendar = createLucideIcon("calendar", __iconNode$1u);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1t = [
  ["path", { d: "M3 3v16a2 2 0 0 0 2 2h16", key: "c24i48" }],
  ["path", { d: "M18 17V9", key: "2bz60n" }],
  ["path", { d: "M13 17V5", key: "1frdt8" }],
  ["path", { d: "M8 17v-3", key: "17ska0" }]
];
const ChartColumn = createLucideIcon("chart-column", __iconNode$1t);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1s = [
  ["path", { d: "M3 3v16a2 2 0 0 0 2 2h16", key: "c24i48" }],
  ["path", { d: "m19 9-5 5-4-4-3 3", key: "2osh9i" }]
];
const ChartLine = createLucideIcon("chart-line", __iconNode$1s);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1r = [["path", { d: "M20 6 9 17l-5-5", key: "1gmf2c" }]];
const Check = createLucideIcon("check", __iconNode$1r);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1q = [["path", { d: "m6 9 6 6 6-6", key: "qrunsl" }]];
const ChevronDown = createLucideIcon("chevron-down", __iconNode$1q);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1p = [["path", { d: "m15 18-6-6 6-6", key: "1wnfg3" }]];
const ChevronLeft = createLucideIcon("chevron-left", __iconNode$1p);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1o = [["path", { d: "m9 18 6-6-6-6", key: "mthhwq" }]];
const ChevronRight = createLucideIcon("chevron-right", __iconNode$1o);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1n = [["path", { d: "m18 15-6-6-6 6", key: "153udz" }]];
const ChevronUp = createLucideIcon("chevron-up", __iconNode$1n);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1m = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["line", { x1: "12", x2: "12", y1: "8", y2: "12", key: "1pkeuh" }],
  ["line", { x1: "12", x2: "12.01", y1: "16", y2: "16", key: "4dfq90" }]
];
const CircleAlert = createLucideIcon("circle-alert", __iconNode$1m);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1l = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "m16 12-4-4-4 4", key: "177agl" }],
  ["path", { d: "M12 16V8", key: "1sbj14" }]
];
const CircleArrowUp = createLucideIcon("circle-arrow-up", __iconNode$1l);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1k = [
  ["path", { d: "M21.801 10A10 10 0 1 1 17 3.335", key: "yps3ct" }],
  ["path", { d: "m9 11 3 3L22 4", key: "1pflzl" }]
];
const CircleCheckBig = createLucideIcon("circle-check-big", __iconNode$1k);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1j = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "m9 12 2 2 4-4", key: "dzmm74" }]
];
const CircleCheck = createLucideIcon("circle-check", __iconNode$1j);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1i = [
  [
    "path",
    {
      d: "M9 9.003a1 1 0 0 1 1.517-.859l4.997 2.997a1 1 0 0 1 0 1.718l-4.997 2.997A1 1 0 0 1 9 14.996z",
      key: "kmsa83"
    }
  ],
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }]
];
const CirclePlay = createLucideIcon("circle-play", __iconNode$1i);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1h = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3", key: "1u773s" }],
  ["path", { d: "M12 17h.01", key: "p32p05" }]
];
const CircleQuestionMark = createLucideIcon("circle-question-mark", __iconNode$1h);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1g = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "m15 9-6 6", key: "1uzhvr" }],
  ["path", { d: "m9 9 6 6", key: "z0biqf" }]
];
const CircleX = createLucideIcon("circle-x", __iconNode$1g);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1f = [
  ["path", { d: "M12 6v6l4 2", key: "mmk7yg" }],
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }]
];
const Clock = createLucideIcon("clock", __iconNode$1f);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1e = [
  ["path", { d: "M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z", key: "p7xjir" }]
];
const Cloud = createLucideIcon("cloud", __iconNode$1e);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1d = [
  ["path", { d: "m18 16 4-4-4-4", key: "1inbqp" }],
  ["path", { d: "m6 8-4 4 4 4", key: "15zrgr" }],
  ["path", { d: "m14.5 4-5 16", key: "e7oirm" }]
];
const CodeXml = createLucideIcon("code-xml", __iconNode$1d);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1c = [
  ["path", { d: "m16 18 6-6-6-6", key: "eg8j8" }],
  ["path", { d: "m8 6-6 6 6 6", key: "ppft3o" }]
];
const Code = createLucideIcon("code", __iconNode$1c);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1b = [
  ["rect", { width: "14", height: "14", x: "8", y: "8", rx: "2", ry: "2", key: "17jyea" }],
  ["path", { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2", key: "zix9uf" }]
];
const Copy = createLucideIcon("copy", __iconNode$1b);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1a = [
  ["path", { d: "M12 20v2", key: "1lh1kg" }],
  ["path", { d: "M12 2v2", key: "tus03m" }],
  ["path", { d: "M17 20v2", key: "1rnc9c" }],
  ["path", { d: "M17 2v2", key: "11trls" }],
  ["path", { d: "M2 12h2", key: "1t8f8n" }],
  ["path", { d: "M2 17h2", key: "7oei6x" }],
  ["path", { d: "M2 7h2", key: "asdhe0" }],
  ["path", { d: "M20 12h2", key: "1q8mjw" }],
  ["path", { d: "M20 17h2", key: "1fpfkl" }],
  ["path", { d: "M20 7h2", key: "1o8tra" }],
  ["path", { d: "M7 20v2", key: "4gnj0m" }],
  ["path", { d: "M7 2v2", key: "1i4yhu" }],
  ["rect", { x: "4", y: "4", width: "16", height: "16", rx: "2", key: "1vbyd7" }],
  ["rect", { x: "8", y: "8", width: "8", height: "8", rx: "1", key: "z9xiuo" }]
];
const Cpu = createLucideIcon("cpu", __iconNode$1a);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$19 = [
  ["rect", { width: "20", height: "14", x: "2", y: "5", rx: "2", key: "ynyp8z" }],
  ["line", { x1: "2", x2: "22", y1: "10", y2: "10", key: "1b3vmo" }]
];
const CreditCard = createLucideIcon("credit-card", __iconNode$19);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$18 = [
  [
    "path",
    {
      d: "M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.246a1 1 0 0 1-.956.734H5.81a1 1 0 0 1-.957-.734L2.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z",
      key: "1vdc57"
    }
  ],
  ["path", { d: "M5 21h14", key: "11awu3" }]
];
const Crown = createLucideIcon("crown", __iconNode$18);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$17 = [
  ["ellipse", { cx: "12", cy: "5", rx: "9", ry: "3", key: "msslwz" }],
  ["path", { d: "M3 5V19A9 3 0 0 0 21 19V5", key: "1wlel7" }],
  ["path", { d: "M3 12A9 3 0 0 0 21 12", key: "mv7ke4" }]
];
const Database = createLucideIcon("database", __iconNode$17);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$16 = [
  ["path", { d: "M12 15V3", key: "m9g1x1" }],
  ["path", { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4", key: "ih7n3h" }],
  ["path", { d: "m7 10 5 5 5-5", key: "brsn70" }]
];
const Download = createLucideIcon("download", __iconNode$16);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$15 = [
  ["circle", { cx: "12", cy: "12", r: "1", key: "41hilf" }],
  ["circle", { cx: "12", cy: "5", r: "1", key: "gxeob9" }],
  ["circle", { cx: "12", cy: "19", r: "1", key: "lyex9k" }]
];
const EllipsisVertical = createLucideIcon("ellipsis-vertical", __iconNode$15);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$14 = [
  ["path", { d: "M15 3h6v6", key: "1q9fwt" }],
  ["path", { d: "M10 14 21 3", key: "gplh6r" }],
  ["path", { d: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6", key: "a6xqqp" }]
];
const ExternalLink = createLucideIcon("external-link", __iconNode$14);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$13 = [
  [
    "path",
    {
      d: "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
      key: "ct8e1f"
    }
  ],
  ["path", { d: "M14.084 14.158a3 3 0 0 1-4.242-4.242", key: "151rxh" }],
  [
    "path",
    {
      d: "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
      key: "13bj9a"
    }
  ],
  ["path", { d: "m2 2 20 20", key: "1ooewy" }]
];
const EyeOff = createLucideIcon("eye-off", __iconNode$13);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$12 = [
  [
    "path",
    {
      d: "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",
      key: "1nclc0"
    }
  ],
  ["circle", { cx: "12", cy: "12", r: "3", key: "1v7zrd" }]
];
const Eye = createLucideIcon("eye", __iconNode$12);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$11 = [
  [
    "path",
    { d: "M12 6a2 2 0 0 1 3.414-1.414l6 6a2 2 0 0 1 0 2.828l-6 6A2 2 0 0 1 12 18z", key: "b19h5q" }
  ],
  [
    "path",
    { d: "M2 6a2 2 0 0 1 3.414-1.414l6 6a2 2 0 0 1 0 2.828l-6 6A2 2 0 0 1 2 18z", key: "h7h5ge" }
  ]
];
const FastForward = createLucideIcon("fast-forward", __iconNode$11);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$10 = [
  ["path", { d: "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z", key: "1rqfz7" }],
  ["path", { d: "M14 2v4a2 2 0 0 0 2 2h4", key: "tnqrlb" }],
  [
    "path",
    { d: "M10 12a1 1 0 0 0-1 1v1a1 1 0 0 1-1 1 1 1 0 0 1 1 1v1a1 1 0 0 0 1 1", key: "1oajmo" }
  ],
  [
    "path",
    { d: "M14 18a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1 1 1 0 0 1-1-1v-1a1 1 0 0 0-1-1", key: "mpwhp6" }
  ]
];
const FileJson = createLucideIcon("file-json", __iconNode$10);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$$ = [
  ["path", { d: "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z", key: "1rqfz7" }],
  ["path", { d: "M14 2v4a2 2 0 0 0 2 2h4", key: "tnqrlb" }],
  ["path", { d: "M10 9H8", key: "b1mrlr" }],
  ["path", { d: "M16 13H8", key: "t4e002" }],
  ["path", { d: "M16 17H8", key: "z1uh3a" }]
];
const FileText = createLucideIcon("file-text", __iconNode$$);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$_ = [
  ["rect", { width: "18", height: "18", x: "3", y: "3", rx: "2", key: "afitv7" }],
  ["path", { d: "M7 3v18", key: "bbkbws" }],
  ["path", { d: "M3 7.5h4", key: "zfgn84" }],
  ["path", { d: "M3 12h18", key: "1i2n21" }],
  ["path", { d: "M3 16.5h4", key: "1230mu" }],
  ["path", { d: "M17 3v18", key: "in4fa5" }],
  ["path", { d: "M17 7.5h4", key: "myr1c1" }],
  ["path", { d: "M17 16.5h4", key: "go4c1d" }]
];
const Film = createLucideIcon("film", __iconNode$_);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$Z = [
  [
    "path",
    {
      d: "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
      key: "usdka0"
    }
  ]
];
const FolderOpen = createLucideIcon("folder-open", __iconNode$Z);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$Y = [
  [
    "path",
    {
      d: "M10 20a1 1 0 0 0 .553.895l2 1A1 1 0 0 0 14 21v-7a2 2 0 0 1 .517-1.341L21.74 4.67A1 1 0 0 0 21 3H3a1 1 0 0 0-.742 1.67l7.225 7.989A2 2 0 0 1 10 14z",
      key: "sc7q7i"
    }
  ]
];
const Funnel = createLucideIcon("funnel", __iconNode$Y);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$X = [
  ["path", { d: "m12 14 4-4", key: "9kzdfg" }],
  ["path", { d: "M3.34 19a10 10 0 1 1 17.32 0", key: "19p75a" }]
];
const Gauge = createLucideIcon("gauge", __iconNode$X);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$W = [
  ["line", { x1: "6", x2: "6", y1: "3", y2: "15", key: "17qcm7" }],
  ["circle", { cx: "18", cy: "6", r: "3", key: "1h7g24" }],
  ["circle", { cx: "6", cy: "18", r: "3", key: "fqmcym" }],
  ["path", { d: "M18 9a9 9 0 0 1-9 9", key: "n2h4wq" }]
];
const GitBranch = createLucideIcon("git-branch", __iconNode$W);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$V = [
  ["circle", { cx: "18", cy: "18", r: "3", key: "1xkwt0" }],
  ["circle", { cx: "6", cy: "6", r: "3", key: "1lh9wr" }],
  ["path", { d: "M13 6h3a2 2 0 0 1 2 2v7", key: "1yeb86" }],
  ["path", { d: "M11 18H8a2 2 0 0 1-2-2V9", key: "19pyzm" }]
];
const GitCompare = createLucideIcon("git-compare", __iconNode$V);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$U = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20", key: "13o1zl" }],
  ["path", { d: "M2 12h20", key: "9i4pu4" }]
];
const Globe = createLucideIcon("globe", __iconNode$U);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$T = [
  ["circle", { cx: "9", cy: "12", r: "1", key: "1vctgf" }],
  ["circle", { cx: "9", cy: "5", r: "1", key: "hp0tcf" }],
  ["circle", { cx: "9", cy: "19", r: "1", key: "fkjjf6" }],
  ["circle", { cx: "15", cy: "12", r: "1", key: "1tmaij" }],
  ["circle", { cx: "15", cy: "5", r: "1", key: "19l28e" }],
  ["circle", { cx: "15", cy: "19", r: "1", key: "f4zoj3" }]
];
const GripVertical = createLucideIcon("grip-vertical", __iconNode$T);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$S = [
  ["line", { x1: "22", x2: "2", y1: "12", y2: "12", key: "1y58io" }],
  [
    "path",
    {
      d: "M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z",
      key: "oot6mr"
    }
  ],
  ["line", { x1: "6", x2: "6.01", y1: "16", y2: "16", key: "sgf278" }],
  ["line", { x1: "10", x2: "10.01", y1: "16", y2: "16", key: "1l4acy" }]
];
const HardDrive = createLucideIcon("hard-drive", __iconNode$S);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$R = [
  [
    "path",
    {
      d: "M2 9.5a5.5 5.5 0 0 1 9.591-3.676.56.56 0 0 0 .818 0A5.49 5.49 0 0 1 22 9.5c0 2.29-1.5 4-3 5.5l-5.492 5.313a2 2 0 0 1-3 .019L5 15c-1.5-1.5-3-3.2-3-5.5",
      key: "mvr1a0"
    }
  ],
  ["path", { d: "M3.22 13H9.5l.5-1 2 4.5 2-7 1.5 3.5h5.27", key: "auskq0" }]
];
const HeartPulse = createLucideIcon("heart-pulse", __iconNode$R);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$Q = [
  ["path", { d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8", key: "1357e3" }],
  ["path", { d: "M3 3v5h5", key: "1xhq8a" }],
  ["path", { d: "M12 7v5l4 2", key: "1fdv2h" }]
];
const History = createLucideIcon("history", __iconNode$Q);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$P = [
  ["path", { d: "M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8", key: "5wwlr5" }],
  [
    "path",
    {
      d: "M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
      key: "r6nss1"
    }
  ]
];
const House = createLucideIcon("house", __iconNode$P);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$O = [
  ["path", { d: "M12 3v12", key: "1x0j5s" }],
  ["path", { d: "m8 11 4 4 4-4", key: "1dohi6" }],
  [
    "path",
    {
      d: "M8 5H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-4",
      key: "1ywtjm"
    }
  ]
];
const Import = createLucideIcon("import", __iconNode$O);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$N = [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "M12 16v-4", key: "1dtifu" }],
  ["path", { d: "M12 8h.01", key: "e9boi3" }]
];
const Info = createLucideIcon("info", __iconNode$N);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$M = [
  ["path", { d: "m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4", key: "g0fldk" }],
  ["path", { d: "m21 2-9.6 9.6", key: "1j0ho8" }],
  ["circle", { cx: "7.5", cy: "15.5", r: "5.5", key: "yqb3hr" }]
];
const Key = createLucideIcon("key", __iconNode$M);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$L = [
  ["path", { d: "M10 8h.01", key: "1r9ogq" }],
  ["path", { d: "M12 12h.01", key: "1mp3jc" }],
  ["path", { d: "M14 8h.01", key: "1primd" }],
  ["path", { d: "M16 12h.01", key: "1l6xoz" }],
  ["path", { d: "M18 8h.01", key: "emo2bl" }],
  ["path", { d: "M6 8h.01", key: "x9i8wu" }],
  ["path", { d: "M7 16h10", key: "wp8him" }],
  ["path", { d: "M8 12h.01", key: "czm47f" }],
  ["rect", { width: "20", height: "16", x: "2", y: "4", rx: "2", key: "18n3k1" }]
];
const Keyboard = createLucideIcon("keyboard", __iconNode$L);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$K = [
  [
    "path",
    {
      d: "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z",
      key: "zw3jo"
    }
  ],
  [
    "path",
    {
      d: "M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12",
      key: "1wduqc"
    }
  ],
  [
    "path",
    {
      d: "M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17",
      key: "kqbvx6"
    }
  ]
];
const Layers = createLucideIcon("layers", __iconNode$K);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$J = [
  ["path", { d: "M9 17H7A5 5 0 0 1 7 7", key: "10o201" }],
  ["path", { d: "M15 7h2a5 5 0 0 1 4 8", key: "1d3206" }],
  ["line", { x1: "8", x2: "12", y1: "12", y2: "12", key: "rvw6j4" }],
  ["line", { x1: "2", x2: "22", y1: "2", y2: "22", key: "a6p6uj" }]
];
const Link2Off = createLucideIcon("link-2-off", __iconNode$J);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$I = [
  ["path", { d: "M9 17H7A5 5 0 0 1 7 7h2", key: "8i5ue5" }],
  ["path", { d: "M15 7h2a5 5 0 1 1 0 10h-2", key: "1b9ql8" }],
  ["line", { x1: "8", x2: "16", y1: "12", y2: "12", key: "1jonct" }]
];
const Link2 = createLucideIcon("link-2", __iconNode$I);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$H = [
  ["path", { d: "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71", key: "1cjeqo" }],
  ["path", { d: "M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71", key: "19qd67" }]
];
const Link = createLucideIcon("link", __iconNode$H);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$G = [["path", { d: "M21 12a9 9 0 1 1-6.219-8.56", key: "13zald" }]];
const LoaderCircle = createLucideIcon("loader-circle", __iconNode$G);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$F = [
  ["rect", { width: "18", height: "11", x: "3", y: "11", rx: "2", ry: "2", key: "1w4ew1" }],
  ["path", { d: "M7 11V7a5 5 0 0 1 10 0v4", key: "fwvmzm" }]
];
const Lock = createLucideIcon("lock", __iconNode$F);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$E = [
  ["path", { d: "m22 7-8.991 5.727a2 2 0 0 1-2.009 0L2 7", key: "132q7q" }],
  ["rect", { x: "2", y: "4", width: "20", height: "16", rx: "2", key: "izxlao" }]
];
const Mail = createLucideIcon("mail", __iconNode$E);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$D = [
  ["path", { d: "M4 5h16", key: "1tepv9" }],
  ["path", { d: "M4 12h16", key: "1lakjw" }],
  ["path", { d: "M4 19h16", key: "1djgab" }]
];
const Menu = createLucideIcon("menu", __iconNode$D);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$C = [
  [
    "path",
    {
      d: "M2.992 16.342a2 2 0 0 1 .094 1.167l-1.065 3.29a1 1 0 0 0 1.236 1.168l3.413-.998a2 2 0 0 1 1.099.092 10 10 0 1 0-4.777-4.719",
      key: "1sd12s"
    }
  ]
];
const MessageCircle = createLucideIcon("message-circle", __iconNode$C);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$B = [
  ["path", { d: "M12 19v3", key: "npa21l" }],
  ["path", { d: "M19 10v2a7 7 0 0 1-14 0v-2", key: "1vc78b" }],
  ["rect", { x: "9", y: "2", width: "6", height: "13", rx: "3", key: "s6n7sd" }]
];
const Mic = createLucideIcon("mic", __iconNode$B);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$A = [["path", { d: "M5 12h14", key: "1ays0h" }]];
const Minus = createLucideIcon("minus", __iconNode$A);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$z = [
  ["rect", { width: "20", height: "14", x: "2", y: "3", rx: "2", key: "48i651" }],
  ["line", { x1: "8", x2: "16", y1: "21", y2: "21", key: "1svkeh" }],
  ["line", { x1: "12", x2: "12", y1: "17", y2: "21", key: "vw1qmm" }]
];
const Monitor = createLucideIcon("monitor", __iconNode$z);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$y = [
  [
    "path",
    {
      d: "M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401",
      key: "kfwtm"
    }
  ]
];
const Moon = createLucideIcon("moon", __iconNode$y);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$x = [
  ["rect", { x: "16", y: "16", width: "6", height: "6", rx: "1", key: "4q2zg0" }],
  ["rect", { x: "2", y: "16", width: "6", height: "6", rx: "1", key: "8cvhb9" }],
  ["rect", { x: "9", y: "2", width: "6", height: "6", rx: "1", key: "1egb70" }],
  ["path", { d: "M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3", key: "1jsf9p" }],
  ["path", { d: "M12 12V8", key: "2874zd" }]
];
const Network = createLucideIcon("network", __iconNode$x);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$w = [
  [
    "path",
    {
      d: "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z",
      key: "1a0edw"
    }
  ],
  ["path", { d: "M12 22V12", key: "d0xqtd" }],
  ["polyline", { points: "3.29 7 12 12 20.71 7", key: "ousv84" }],
  ["path", { d: "m7.5 4.27 9 5.15", key: "1c824w" }]
];
const Package = createLucideIcon("package", __iconNode$w);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$v = [
  [
    "path",
    {
      d: "M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z",
      key: "e79jfc"
    }
  ],
  ["circle", { cx: "13.5", cy: "6.5", r: ".5", fill: "currentColor", key: "1okk4w" }],
  ["circle", { cx: "17.5", cy: "10.5", r: ".5", fill: "currentColor", key: "f64h9f" }],
  ["circle", { cx: "6.5", cy: "12.5", r: ".5", fill: "currentColor", key: "qy21gx" }],
  ["circle", { cx: "8.5", cy: "7.5", r: ".5", fill: "currentColor", key: "fotxhn" }]
];
const Palette = createLucideIcon("palette", __iconNode$v);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$u = [
  ["rect", { width: "18", height: "18", x: "3", y: "3", rx: "2", key: "afitv7" }],
  ["path", { d: "M3 9h18", key: "1pudct" }],
  ["path", { d: "M9 21V9", key: "1oto5p" }]
];
const PanelsTopLeft = createLucideIcon("panels-top-left", __iconNode$u);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$t = [
  ["rect", { x: "14", y: "3", width: "5", height: "18", rx: "1", key: "kaeet6" }],
  ["rect", { x: "5", y: "3", width: "5", height: "18", rx: "1", key: "1wsw3u" }]
];
const Pause = createLucideIcon("pause", __iconNode$t);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$s = [
  [
    "path",
    {
      d: "M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z",
      key: "10ikf1"
    }
  ]
];
const Play = createLucideIcon("play", __iconNode$s);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$r = [
  ["path", { d: "M5 12h14", key: "1ays0h" }],
  ["path", { d: "M12 5v14", key: "s699le" }]
];
const Plus = createLucideIcon("plus", __iconNode$r);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$q = [
  [
    "path",
    {
      d: "M15.39 4.39a1 1 0 0 0 1.68-.474 2.5 2.5 0 1 1 3.014 3.015 1 1 0 0 0-.474 1.68l1.683 1.682a2.414 2.414 0 0 1 0 3.414L19.61 15.39a1 1 0 0 1-1.68-.474 2.5 2.5 0 1 0-3.014 3.015 1 1 0 0 1 .474 1.68l-1.683 1.682a2.414 2.414 0 0 1-3.414 0L8.61 19.61a1 1 0 0 0-1.68.474 2.5 2.5 0 1 1-3.014-3.015 1 1 0 0 0 .474-1.68l-1.683-1.682a2.414 2.414 0 0 1 0-3.414L4.39 8.61a1 1 0 0 1 1.68.474 2.5 2.5 0 1 0 3.014-3.015 1 1 0 0 1-.474-1.68l1.683-1.682a2.414 2.414 0 0 1 3.414 0z",
      key: "w46dr5"
    }
  ]
];
const Puzzle = createLucideIcon("puzzle", __iconNode$q);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$p = [
  ["path", { d: "M16.247 7.761a6 6 0 0 1 0 8.478", key: "1fwjs5" }],
  ["path", { d: "M19.075 4.933a10 10 0 0 1 0 14.134", key: "ehdyv1" }],
  ["path", { d: "M4.925 19.067a10 10 0 0 1 0-14.134", key: "1q22gi" }],
  ["path", { d: "M7.753 16.239a6 6 0 0 1 0-8.478", key: "r2q7qm" }],
  ["circle", { cx: "12", cy: "12", r: "2", key: "1c9p78" }]
];
const Radio = createLucideIcon("radio", __iconNode$p);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$o = [
  ["path", { d: "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8", key: "v9h5vc" }],
  ["path", { d: "M21 3v5h-5", key: "1q7to0" }],
  ["path", { d: "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16", key: "3uifl3" }],
  ["path", { d: "M8 16H3v5", key: "1cv678" }]
];
const RefreshCw = createLucideIcon("refresh-cw", __iconNode$o);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$n = [
  [
    "path",
    {
      d: "M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z",
      key: "m3kijz"
    }
  ],
  [
    "path",
    {
      d: "m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z",
      key: "1fmvmk"
    }
  ],
  ["path", { d: "M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0", key: "1f8sc4" }],
  ["path", { d: "M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5", key: "qeys4" }]
];
const Rocket = createLucideIcon("rocket", __iconNode$n);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$m = [
  ["path", { d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8", key: "1357e3" }],
  ["path", { d: "M3 3v5h5", key: "1xhq8a" }]
];
const RotateCcw = createLucideIcon("rotate-ccw", __iconNode$m);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$l = [
  [
    "path",
    {
      d: "M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
      key: "1c8476"
    }
  ],
  ["path", { d: "M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7", key: "1ydtos" }],
  ["path", { d: "M7 3v4a1 1 0 0 0 1 1h7", key: "t51u73" }]
];
const Save = createLucideIcon("save", __iconNode$l);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$k = [
  ["path", { d: "m21 21-4.34-4.34", key: "14j7rj" }],
  ["circle", { cx: "11", cy: "11", r: "8", key: "4ej97u" }]
];
const Search = createLucideIcon("search", __iconNode$k);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$j = [
  ["rect", { width: "20", height: "8", x: "2", y: "2", rx: "2", ry: "2", key: "ngkwjq" }],
  ["rect", { width: "20", height: "8", x: "2", y: "14", rx: "2", ry: "2", key: "iecqi9" }],
  ["line", { x1: "6", x2: "6.01", y1: "6", y2: "6", key: "16zg32" }],
  ["line", { x1: "6", x2: "6.01", y1: "18", y2: "18", key: "nzw8ys" }]
];
const Server = createLucideIcon("server", __iconNode$j);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$i = [
  [
    "path",
    {
      d: "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
      key: "1i5ecw"
    }
  ],
  ["circle", { cx: "12", cy: "12", r: "3", key: "1v7zrd" }]
];
const Settings = createLucideIcon("settings", __iconNode$i);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$h = [
  ["circle", { cx: "18", cy: "5", r: "3", key: "gq8acd" }],
  ["circle", { cx: "6", cy: "12", r: "3", key: "w7nqdw" }],
  ["circle", { cx: "18", cy: "19", r: "3", key: "1xt0gg" }],
  ["line", { x1: "8.59", x2: "15.42", y1: "13.51", y2: "17.49", key: "47mynk" }],
  ["line", { x1: "15.41", x2: "8.59", y1: "6.51", y2: "10.49", key: "1n3mei" }]
];
const Share2 = createLucideIcon("share-2", __iconNode$h);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$g = [
  [
    "path",
    {
      d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z",
      key: "oel41y"
    }
  ]
];
const Shield = createLucideIcon("shield", __iconNode$g);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$f = [
  ["path", { d: "M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7", key: "1m0v6g" }],
  [
    "path",
    {
      d: "M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z",
      key: "ohrbg2"
    }
  ]
];
const SquarePen = createLucideIcon("square-pen", __iconNode$f);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$e = [
  [
    "path",
    {
      d: "M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",
      key: "r04s7s"
    }
  ]
];
const Star = createLucideIcon("star", __iconNode$e);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$d = [
  ["path", { d: "M15 21v-5a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v5", key: "slp6dd" }],
  [
    "path",
    {
      d: "M17.774 10.31a1.12 1.12 0 0 0-1.549 0 2.5 2.5 0 0 1-3.451 0 1.12 1.12 0 0 0-1.548 0 2.5 2.5 0 0 1-3.452 0 1.12 1.12 0 0 0-1.549 0 2.5 2.5 0 0 1-3.77-3.248l2.889-4.184A2 2 0 0 1 7 2h10a2 2 0 0 1 1.653.873l2.895 4.192a2.5 2.5 0 0 1-3.774 3.244",
      key: "o0xfot"
    }
  ],
  ["path", { d: "M4 10.95V19a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8.05", key: "wn3emo" }]
];
const Store = createLucideIcon("store", __iconNode$d);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$c = [
  ["circle", { cx: "12", cy: "12", r: "4", key: "4exip2" }],
  ["path", { d: "M12 2v2", key: "tus03m" }],
  ["path", { d: "M12 20v2", key: "1lh1kg" }],
  ["path", { d: "m4.93 4.93 1.41 1.41", key: "149t6j" }],
  ["path", { d: "m17.66 17.66 1.41 1.41", key: "ptbguv" }],
  ["path", { d: "M2 12h2", key: "1t8f8n" }],
  ["path", { d: "M20 12h2", key: "1q8mjw" }],
  ["path", { d: "m6.34 17.66-1.41 1.41", key: "1m8zz5" }],
  ["path", { d: "m19.07 4.93-1.41 1.41", key: "1shlcs" }]
];
const Sun = createLucideIcon("sun", __iconNode$c);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$b = [
  ["path", { d: "M12 3v18", key: "108xh3" }],
  ["rect", { width: "18", height: "18", x: "3", y: "3", rx: "2", key: "afitv7" }],
  ["path", { d: "M3 9h18", key: "1pudct" }],
  ["path", { d: "M3 15h18", key: "5xshup" }]
];
const Table = createLucideIcon("table", __iconNode$b);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$a = [
  ["path", { d: "M14.5 2v17.5c0 1.4-1.1 2.5-2.5 2.5c-1.4 0-2.5-1.1-2.5-2.5V2", key: "125lnx" }],
  ["path", { d: "M8.5 2h7", key: "csnxdl" }],
  ["path", { d: "M14.5 16h-5", key: "1ox875" }]
];
const TestTube = createLucideIcon("test-tube", __iconNode$a);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$9 = [
  ["path", { d: "M10 11v6", key: "nco0om" }],
  ["path", { d: "M14 11v6", key: "outv1u" }],
  ["path", { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6", key: "miytrc" }],
  ["path", { d: "M3 6h18", key: "d0wm0j" }],
  ["path", { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2", key: "e791ji" }]
];
const Trash2 = createLucideIcon("trash-2", __iconNode$9);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$8 = [
  ["path", { d: "M16 7h6v6", key: "box55l" }],
  ["path", { d: "m22 7-8.5 8.5-5-5L2 17", key: "1t1m79" }]
];
const TrendingUp = createLucideIcon("trending-up", __iconNode$8);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$7 = [
  [
    "path",
    {
      d: "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
      key: "wmoenq"
    }
  ],
  ["path", { d: "M12 9v4", key: "juzpu7" }],
  ["path", { d: "M12 17h.01", key: "p32p05" }]
];
const TriangleAlert = createLucideIcon("triangle-alert", __iconNode$7);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$6 = [
  ["path", { d: "M12 3v12", key: "1x0j5s" }],
  ["path", { d: "m17 8-5-5-5 5", key: "7q97r8" }],
  ["path", { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4", key: "ih7n3h" }]
];
const Upload = createLucideIcon("upload", __iconNode$6);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$5 = [
  ["path", { d: "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2", key: "975kel" }],
  ["circle", { cx: "12", cy: "7", r: "4", key: "17ys0d" }]
];
const User = createLucideIcon("user", __iconNode$5);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$4 = [
  ["path", { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2", key: "1yyitq" }],
  ["path", { d: "M16 3.128a4 4 0 0 1 0 7.744", key: "16gr8j" }],
  ["path", { d: "M22 21v-2a4 4 0 0 0-3-3.87", key: "kshegd" }],
  ["circle", { cx: "9", cy: "7", r: "4", key: "nufk8" }]
];
const Users = createLucideIcon("users", __iconNode$4);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$3 = [
  ["path", { d: "M12 20h.01", key: "zekei9" }],
  ["path", { d: "M8.5 16.429a5 5 0 0 1 7 0", key: "1bycff" }],
  ["path", { d: "M5 12.859a10 10 0 0 1 5.17-2.69", key: "1dl1wf" }],
  ["path", { d: "M19 12.859a10 10 0 0 0-2.007-1.523", key: "4k23kn" }],
  ["path", { d: "M2 8.82a15 15 0 0 1 4.177-2.643", key: "1grhjp" }],
  ["path", { d: "M22 8.82a15 15 0 0 0-11.288-3.764", key: "z3jwby" }],
  ["path", { d: "m2 2 20 20", key: "1ooewy" }]
];
const WifiOff = createLucideIcon("wifi-off", __iconNode$3);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$2 = [
  ["path", { d: "M12 20h.01", key: "zekei9" }],
  ["path", { d: "M2 8.82a15 15 0 0 1 20 0", key: "dnpr2z" }],
  ["path", { d: "M5 12.859a10 10 0 0 1 14 0", key: "1x1e6c" }],
  ["path", { d: "M8.5 16.429a5 5 0 0 1 7 0", key: "1bycff" }]
];
const Wifi = createLucideIcon("wifi", __iconNode$2);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode$1 = [
  ["path", { d: "M18 6 6 18", key: "1bl5f8" }],
  ["path", { d: "m6 6 12 12", key: "d8bk6v" }]
];
const X = createLucideIcon("x", __iconNode$1);
/**
 * @license lucide-react v0.544.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */
const __iconNode = [
  [
    "path",
    {
      d: "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z",
      key: "1xq2db"
    }
  ]
];
const Zap = createLucideIcon("zap", __iconNode);
const coreBrandTheme = {
  id: "core-brand",
  name: "MockForge Core",
  description: "Professional orange-themed design with clean contrast",
  preview: {
    primary: "#D35400",
    secondary: "#FAFBFC",
    accent: "#10B981"
  },
  colors: {
    light: {
      // Background colors
      "--background": "0 0% 100%",
      "--card": "0 0% 100%",
      "--card-foreground": "220 15% 15%",
      "--popover": "0 0% 100%",
      "--popover-foreground": "220 15% 15%",
      "--primary": "24 86% 42%",
      "--primary-foreground": "0 0% 100%",
      "--secondary": "217.2 32.6% 17.5%",
      "--secondary-foreground": "210 40% 98%",
      "--muted": "210 40% 96.1%",
      "--muted-foreground": "215.4 16.3% 46.9%",
      "--accent": "210 40% 96.1%",
      "--accent-foreground": "222.2 47.4% 11.2%",
      "--destructive": "0 84% 50%",
      "--destructive-foreground": "0 0% 98%",
      "--border": "214.3 31.8% 91.4%",
      "--input": "214.3 31.8% 91.4%",
      "--ring": "215 20% 65%",
      "--brand": "24 86% 42%",
      "--brand-50": "24 100% 97%",
      "--brand-100": "24 95% 92%",
      "--brand-200": "24 90% 84%",
      "--brand-300": "24 85% 70%",
      "--brand-400": "24 85% 55%",
      "--brand-500": "24 86% 42%",
      "--brand-600": "24 88% 36%",
      "--brand-700": "24 92% 30%",
      "--brand-800": "24 95% 24%",
      "--brand-900": "24 98% 18%",
      "--success": "142 76% 36%",
      "--success-50": "142 100% 97%",
      "--success-100": "142 90% 92%",
      "--success-500": "142 76% 36%",
      "--success-600": "142 78% 32%",
      "--warning": "42 96% 50%",
      "--warning-50": "42 100% 96%",
      "--warning-100": "42 95% 90%",
      "--warning-500": "42 96% 50%",
      "--warning-600": "42 98% 45%",
      "--danger": "0 84% 50%",
      "--danger-50": "0 100% 97%",
      "--danger-100": "0 95% 92%",
      "--danger-500": "0 84% 50%",
      "--danger-600": "0 86% 45%",
      "--info": "217 91% 60%",
      "--info-50": "217 100% 97%",
      "--info-100": "217 95% 92%",
      "--info-500": "217 91% 60%",
      "--info-600": "217 93% 55%",
      "--bg-primary": "0 0% 100%",
      "--bg-secondary": "210 40% 98%",
      "--bg-tertiary": "210 40% 96%",
      "--bg-overlay": "0 0% 0% / 0.5",
      "--text-primary": "220 15% 15%",
      "--text-secondary": "220 10% 40%",
      "--text-tertiary": "220 10% 55%",
      "--text-inverse": "0 0% 100%",
      "--radius": "12px"
    },
    dark: {
      "--background": "220 15% 7%",
      "--card": "222 15% 9%",
      "--card-foreground": "210 20% 98%",
      "--popover": "222 16% 11%",
      "--popover-foreground": "210 20% 98%",
      "--primary": "24 86% 52%",
      "--primary-foreground": "220 15% 7%",
      "--secondary": "219 13% 18%",
      "--secondary-foreground": "210 20% 98%",
      "--muted": "219 13% 18%",
      "--muted-foreground": "215 16% 72%",
      "--accent": "219 13% 18%",
      "--accent-foreground": "210 20% 98%",
      "--destructive": "0 84% 60%",
      "--destructive-foreground": "220 15% 7%",
      "--border": "215 15% 20%",
      "--input": "215 15% 20%",
      "--ring": "24 86% 52%",
      "--brand": "24 86% 52%",
      "--brand-50": "24 15% 12%",
      "--brand-100": "24 20% 18%",
      "--brand-200": "24 25% 24%",
      "--brand-300": "24 30% 30%",
      "--brand-400": "24 40% 40%",
      "--brand-500": "24 86% 52%",
      "--brand-600": "24 88% 48%",
      "--brand-700": "24 92% 42%",
      "--brand-800": "24 95% 36%",
      "--brand-900": "24 98% 24%",
      "--success": "142 76% 48%",
      "--success-50": "142 15% 12%",
      "--success-100": "142 20% 18%",
      "--success-500": "142 76% 48%",
      "--success-600": "142 78% 42%",
      "--warning": "42 96% 60%",
      "--warning-50": "42 15% 12%",
      "--warning-100": "42 20% 18%",
      "--warning-500": "42 96% 60%",
      "--warning-600": "42 98% 55%",
      "--danger": "0 84% 60%",
      "--danger-50": "0 15% 12%",
      "--danger-100": "0 20% 18%",
      "--danger-500": "0 84% 60%",
      "--danger-600": "0 86% 55%",
      "--info": "217 91% 65%",
      "--info-50": "217 15% 12%",
      "--info-100": "217 20% 18%",
      "--info-500": "217 91% 65%",
      "--info-600": "217 93% 60%",
      "--bg-primary": "222 15% 9%",
      "--bg-secondary": "220 15% 7%",
      "--bg-tertiary": "219 13% 12%",
      "--bg-overlay": "0 0% 0% / 0.7",
      "--text-primary": "210 20% 98%",
      "--text-secondary": "215 16% 72%",
      "--text-tertiary": "215 12% 55%",
      "--text-inverse": "220 15% 7%",
      "--radius": "12px"
    }
  }
};
const professionalBlueTheme = {
  id: "professional-blue",
  name: "Professional Blue",
  description: "Clean corporate blue for business environments",
  preview: {
    primary: "#2563EB",
    secondary: "#F8FAFC",
    accent: "#10B981"
  },
  colors: {
    light: {
      "--background": "0 0% 100%",
      "--card": "0 0% 100%",
      "--card-foreground": "222.2 84% 4.9%",
      "--popover": "0 0% 100%",
      "--popover-foreground": "222.2 84% 4.9%",
      "--primary": "221.2 83.2% 53.3%",
      "--primary-foreground": "210 40% 98%",
      "--secondary": "210 40% 96%",
      "--secondary-foreground": "222.2 84% 4.9%",
      "--muted": "210 40% 96%",
      "--muted-foreground": "215.4 16.3% 46.9%",
      "--accent": "210 40% 96%",
      "--accent-foreground": "222.2 84% 4.9%",
      "--destructive": "0 84.2% 60.2%",
      "--destructive-foreground": "210 40% 98%",
      "--border": "214.3 31.8% 91.4%",
      "--input": "214.3 31.8% 91.4%",
      "--ring": "221.2 83.2% 53.3%",
      "--brand": "221.2 83.2% 53.3%",
      "--brand-50": "214.3 31.8% 91.4%",
      "--brand-100": "214.3 31.8% 91.4%",
      "--brand-200": "209.4 31.8% 84%",
      "--brand-300": "209.4 34% 70%",
      "--brand-400": "217.2 32% 54%",
      "--brand-500": "221.2 83.2% 53.3%",
      "--brand-600": "217.2 91.2% 60%",
      "--brand-700": "217.2 91.2% 42%",
      "--brand-800": "217.2 93.2% 30%",
      "--brand-900": "217.2 93.2% 18%",
      "--success": "142 76.2% 36.3%",
      "--success-50": "138 62.8% 94.6%",
      "--success-100": "134 60.3% 84%",
      "--success-500": "142 76.2% 36.3%",
      "--success-600": "142 71% 45%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "53.8 91.8% 94.6%",
      "--warning-100": "49.8 91.7% 84%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 84.2% 60.2%",
      "--danger-50": "0 85.7% 97.3%",
      "--danger-100": "0 74.7% 84%",
      "--danger-500": "0 84.2% 60.2%",
      "--danger-600": "0 72% 51%",
      "--info": "199.4 89% 48.3%",
      "--info-50": "199.4 89% 95%",
      "--info-100": "197.4 89% 84%",
      "--info-500": "199.4 89% 48.3%",
      "--info-600": "202.4 89% 48%",
      "--bg-primary": "0 0% 100%",
      "--bg-secondary": "210 40% 98%",
      "--bg-tertiary": "210 40% 96%",
      "--bg-overlay": "0 0% 0% / 0.5",
      "--text-primary": "222.2 84% 4.9%",
      "--text-secondary": "215.4 16.3% 46.9%",
      "--text-tertiary": "215 13.8% 34.1%",
      "--text-inverse": "0 0% 100%",
      "--radius": "12px"
    },
    dark: {
      "--background": "222.2 84% 4.9%",
      "--card": "222.2 84% 4.9%",
      "--card-foreground": "210 40% 98%",
      "--popover": "222.2 84% 4.9%",
      "--popover-foreground": "210 40% 98%",
      "--primary": "217.2 91.2% 59.8%",
      "--primary-foreground": "222.2 84% 4.9%",
      "--secondary": "217.2 32.6% 17.5%",
      "--secondary-foreground": "210 40% 98%",
      "--muted": "217.2 32.6% 17.5%",
      "--muted-foreground": "215 20.2% 65.1%",
      "--accent": "217.2 32.6% 17.5%",
      "--accent-foreground": "210 40% 98%",
      "--destructive": "0 62.8% 30.6%",
      "--destructive-foreground": "210 40% 98%",
      "--border": "217.2 32.6% 17.5%",
      "--input": "217.2 32.6% 17.5%",
      "--ring": "224.3 76.3% 94.1%",
      "--brand": "217.2 91.2% 59.8%",
      "--brand-50": "210 5% 8%",
      "--brand-100": "214.3 31.8% 10%",
      "--brand-200": "217.2 32.6% 17.5%",
      "--brand-300": "216.5 41% 25%",
      "--brand-400": "217.2 32% 35%",
      "--brand-500": "217.2 91.2% 59.8%",
      "--brand-600": "217.2 91.2% 46%",
      "--brand-700": "217.2 91.2% 35%",
      "--brand-800": "217.2 91.2% 25%",
      "--brand-900": "217.2 91.2% 15%",
      "--success": "142 76.2% 36.3%",
      "--success-50": "120 5% 8%",
      "--success-100": "125 60% 10%",
      "--success-500": "142 76.2% 36.3%",
      "--success-600": "142 71% 45%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "36 4% 8%",
      "--warning-100": "36 93% 10%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 62.8% 30.6%",
      "--danger-50": "0 5% 6%",
      "--danger-100": "0 45% 10%",
      "--danger-500": "0 62.8% 30.6%",
      "--danger-600": "0 72% 51%",
      "--info": "199.4 89% 48.3%",
      "--info-50": "180 4% 8%",
      "--info-100": "180 89% 10%",
      "--info-500": "199.4 89% 48.3%",
      "--info-600": "202.4 89% 48%",
      "--bg-primary": "222.2 84% 4.9%",
      "--bg-secondary": "217.2 32.6% 17.5%",
      "--bg-tertiary": "217.2 32.6% 17.5%",
      "--bg-overlay": "0 0% 0% / 0.7",
      "--text-primary": "210 40% 98%",
      "--text-secondary": "215 20.2% 65.1%",
      "--text-tertiary": "217.2 10.6% 64.9%",
      "--text-inverse": "222.2 84% 4.9%",
      "--radius": "12px"
    }
  }
};
const forestGreenTheme = {
  id: "forest-green",
  name: "Forest Green",
  description: "Natural green palette inspired by forest landscapes",
  preview: {
    primary: "#16A34A",
    secondary: "#F0FDF4",
    accent: "#DC2626"
  },
  colors: {
    light: {
      "--background": "0 0% 100%",
      "--card": "0 0% 100%",
      "--card-foreground": "222.2 84% 4.9%",
      "--popover": "0 0% 100%",
      "--popover-foreground": "222.2 84% 4.9%",
      "--primary": "142 76% 37%",
      "--primary-foreground": "210 40% 98%",
      "--secondary": "141 84% 96%",
      "--secondary-foreground": "222.2 84% 4.9%",
      "--muted": "141 84% 96%",
      "--muted-foreground": "215.4 16.3% 46.9%",
      "--accent": "141 84% 96%",
      "--accent-foreground": "222.2 84% 4.9%",
      "--destructive": "0 84.2% 60.2%",
      "--destructive-foreground": "210 40% 98%",
      "--border": "141 84% 93%",
      "--input": "141 84% 93%",
      "--ring": "142 76% 37%",
      "--brand": "142 76% 37%",
      "--brand-50": "137 88% 96%",
      "--brand-100": "136 82% 93%",
      "--brand-200": "141 64% 84%",
      "--brand-300": "142 69% 70%",
      "--brand-400": "142 78% 55%",
      "--brand-500": "142 76% 37%",
      "--brand-600": "142 76% 26%",
      "--brand-700": "142 76% 20%",
      "--brand-800": "142 76% 15%",
      "--brand-900": "142 76% 10%",
      "--success": "142 76% 37%",
      "--success-50": "141 84% 96%",
      "--success-100": "141 84% 93%",
      "--success-500": "142 76% 37%",
      "--success-600": "142 76% 26%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "53.8 91.8% 94.6%",
      "--warning-100": "49.8 91.7% 84%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 84.2% 60.2%",
      "--danger-50": "0 85.7% 97.3%",
      "--danger-100": "0 74.7% 84%",
      "--danger-500": "0 84.2% 60.2%",
      "--danger-600": "0 72% 51%",
      "--info": "221.2 83.2% 53.3%",
      "--info-50": "214.3 31.8% 91.4%",
      "--info-100": "214.3 31.8% 91.4%",
      "--info-500": "221.2 83.2% 53.3%",
      "--info-600": "217.2 91.2% 60%",
      "--bg-primary": "0 0% 100%",
      "--bg-secondary": "141 84% 96%",
      "--bg-tertiary": "141 84% 93%",
      "--bg-overlay": "0 0% 0% / 0.5",
      "--text-primary": "222.2 84% 4.9%",
      "--text-secondary": "215.4 16.3% 46.9%",
      "--text-tertiary": "215 13.8% 34.1%",
      "--text-inverse": "0 0% 100%",
      "--radius": "12px"
    },
    dark: {
      "--background": "120 10% 3.9%",
      "--card": "120 10% 3.9%",
      "--card-foreground": "120 9% 98%",
      "--popover": "120 10% 3.9%",
      "--popover-foreground": "120 9% 98%",
      "--primary": "142 76% 37%",
      "--primary-foreground": "120 10% 3.9%",
      "--secondary": "120 5% 10%",
      "--secondary-foreground": "120 9% 98%",
      "--muted": "120 5% 10%",
      "--muted-foreground": "120 5% 65%",
      "--accent": "120 5% 10%",
      "--accent-foreground": "120 9% 98%",
      "--destructive": "0 62.8% 30.6%",
      "--destructive-foreground": "120 9% 98%",
      "--border": "120 3.7% 15.9%",
      "--input": "120 3.7% 15.9%",
      "--ring": "142 76% 37%",
      "--brand": "142 76% 37%",
      "--brand-50": "120 3% 8%",
      "--brand-100": "120 5% 10%",
      "--brand-200": "120 5% 15%",
      "--brand-300": "120 5% 20%",
      "--brand-400": "120 5% 25%",
      "--brand-500": "142 76% 37%",
      "--brand-600": "142 76% 26%",
      "--brand-700": "142 76% 20%",
      "--brand-800": "142 76% 15%",
      "--brand-900": "142 76% 10%",
      "--success": "142 76% 37%",
      "--success-50": "120 5% 10%",
      "--success-100": "120 5% 15%",
      "--success-500": "142 76% 37%",
      "--success-600": "142 76% 26%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "36 4% 8%",
      "--warning-100": "36 93% 10%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 62.8% 30.6%",
      "--danger-50": "0 5% 6%",
      "--danger-100": "0 45% 10%",
      "--danger-500": "0 62.8% 30.6%",
      "--danger-600": "0 72% 51%",
      "--info": "221.2 83.2% 53.3%",
      "--info-50": "214.3 31% 8%",
      "--info-100": "214.3 31% 12%",
      "--info-500": "221.2 83.2% 53.3%",
      "--info-600": "217.2 91.2% 60%",
      "--bg-primary": "120 10% 3.9%",
      "--bg-secondary": "120 5% 10%",
      "--bg-tertiary": "120 3.7% 15.9%",
      "--bg-overlay": "0 0% 0% / 0.7",
      "--text-primary": "120 9% 98%",
      "--text-secondary": "120 5% 65%",
      "--text-tertiary": "120 3% 50%",
      "--text-inverse": "120 10% 3.9%",
      "--radius": "12px"
    }
  }
};
const twilightPurpleTheme = {
  id: "twilight-purple",
  name: "Twilight Purple",
  description: "Elegant purple gradient inspired by sunset skies",
  preview: {
    primary: "#8B5CF6",
    secondary: "#FAF5FF",
    accent: "#F59E0B"
  },
  colors: {
    light: {
      "--background": "0 0% 100%",
      "--card": "0 0% 100%",
      "--card-foreground": "222.2 84% 4.9%",
      "--popover": "0 0% 100%",
      "--popover-foreground": "222.2 84% 4.9%",
      "--primary": "255 92% 61%",
      "--primary-foreground": "210 40% 98%",
      "--secondary": "255 92% 96%",
      "--secondary-foreground": "222.2 84% 4.9%",
      "--muted": "255 92% 96%",
      "--muted-foreground": "215.4 16.3% 46.9%",
      "--accent": "255 92% 96%",
      "--accent-foreground": "222.2 84% 4.9%",
      "--destructive": "0 84.2% 60.2%",
      "--destructive-foreground": "210 40% 98%",
      "--border": "255 92% 93%",
      "--input": "255 92% 93%",
      "--ring": "255 92% 61%",
      "--brand": "255 92% 61%",
      "--brand-50": "261 91% 96%",
      "--brand-100": "261 91% 93%",
      "--brand-200": "261 91% 84%",
      "--brand-300": "262 83% 75%",
      "--brand-400": "261 91% 65%",
      "--brand-500": "255 92% 61%",
      "--brand-600": "262 83% 50%",
      "--brand-700": "262 83% 35%",
      "--brand-800": "262 83% 25%",
      "--brand-900": "262 83% 15%",
      "--success": "142 76.2% 36.3%",
      "--success-50": "138 62.8% 94.6%",
      "--success-100": "134 60.3% 84%",
      "--success-500": "142 76.2% 36.3%",
      "--success-600": "142 71% 45%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "53.8 91.8% 94.6%",
      "--warning-100": "49.8 91.7% 84%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 84.2% 60.2%",
      "--danger-50": "0 85.7% 97.3%",
      "--danger-100": "0 74.7% 84%",
      "--danger-500": "0 84.2% 60.2%",
      "--danger-600": "0 72% 51%",
      "--info": "221.2 83.2% 53.3%",
      "--info-50": "214.3 31.8% 91.4%",
      "--info-100": "214.3 31.8% 84%",
      "--info-500": "221.2 83.2% 53.3%",
      "--info-600": "217.2 91.2% 60%",
      "--bg-primary": "0 0% 100%",
      "--bg-secondary": "255 92% 96%",
      "--bg-tertiary": "261 91% 93%",
      "--bg-overlay": "0 0% 0% / 0.5",
      "--text-primary": "222.2 84% 4.9%",
      "--text-secondary": "215.4 16.3% 46.9%",
      "--text-tertiary": "215 13.8% 34.1%",
      "--text-inverse": "0 0% 100%",
      "--radius": "12px"
    },
    dark: {
      "--background": "255 10% 3.9%",
      "--card": "255 10% 3.9%",
      "--card-foreground": "255 10% 98%",
      "--popover": "255 10% 3.9%",
      "--popover-foreground": "255 10% 98%",
      "--primary": "255 92% 61%",
      "--primary-foreground": "255 10% 3.9%",
      "--secondary": "255 20% 10%",
      "--secondary-foreground": "255 10% 98%",
      "--muted": "255 20% 10%",
      "--muted-foreground": "255 10% 65%",
      "--accent": "255 20% 10%",
      "--accent-foreground": "255 10% 98%",
      "--destructive": "0 62.8% 30.6%",
      "--destructive-foreground": "255 10% 98%",
      "--border": "255 10% 15.9%",
      "--input": "255 10% 15.9%",
      "--ring": "255 92% 61%",
      "--brand": "255 92% 61%",
      "--brand-50": "255 5% 8%",
      "--brand-100": "255 15% 10%",
      "--brand-200": "255 20% 15%",
      "--brand-300": "255 20% 20%",
      "--brand-400": "255 25% 25%",
      "--brand-500": "255 92% 61%",
      "--brand-600": "262 83% 50%",
      "--brand-700": "262 83% 35%",
      "--brand-800": "262 83% 25%",
      "--brand-900": "262 83% 15%",
      "--success": "142 76.2% 36.3%",
      "--success-50": "125 15% 8%",
      "--success-100": "125 20% 12%",
      "--success-500": "142 76.2% 36.3%",
      "--success-600": "142 71% 45%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "36 4% 8%",
      "--warning-100": "36 93% 10%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 62.8% 30.6%",
      "--danger-50": "0 5% 6%",
      "--danger-100": "0 45% 10%",
      "--danger-500": "0 62.8% 30.6%",
      "--danger-600": "0 72% 51%",
      "--info": "221.2 83.2% 53.3%",
      "--info-50": "214.3 31% 8%",
      "--info-100": "214.3 31% 12%",
      "--info-500": "221.2 83.2% 53.3%",
      "--info-600": "217.2 91.2% 60%",
      "--bg-primary": "255 10% 3.9%",
      "--bg-secondary": "255 20% 10%",
      "--bg-tertiary": "255 10% 15.9%",
      "--bg-overlay": "0 0% 0% / 0.7",
      "--text-primary": "255 10% 98%",
      "--text-secondary": "255 10% 65%",
      "--text-tertiary": "255 10% 50%",
      "--text-inverse": "255 10% 3.9%",
      "--radius": "12px"
    }
  }
};
const oceanDeepTheme = {
  id: "ocean-deep",
  name: "Ocean Deep",
  description: "Deep blue waves with teal accents for a nautical feel",
  preview: {
    primary: "#0EA5E9",
    secondary: "#F0F9FF",
    accent: "#EC4899"
  },
  colors: {
    light: {
      "--background": "0 0% 100%",
      "--card": "0 0% 100%",
      "--card-foreground": "222.2 84% 4.9%",
      "--popover": "0 0% 100%",
      "--popover-foreground": "222.2 84% 4.9%",
      "--primary": "199 89% 48%",
      "--primary-foreground": "210 40% 98%",
      "--secondary": "204 100% 97%",
      "--secondary-foreground": "222.2 84% 4.9%",
      "--muted": "204 100% 97%",
      "--muted-foreground": "215.4 16.3% 46.9%",
      "--accent": "204 100% 97%",
      "--accent-foreground": "222.2 84% 4.9%",
      "--destructive": "0 84.2% 60.2%",
      "--destructive-foreground": "210 40% 98%",
      "--border": "204 94% 94%",
      "--input": "204 94% 94%",
      "--ring": "199 89% 48%",
      "--brand": "199 89% 48%",
      "--brand-50": "204 100% 97%",
      "--brand-100": "204 100% 97%",
      "--brand-200": "204 94% 90%",
      "--brand-300": "204 94% 82%",
      "--brand-400": "202 82% 65%",
      "--brand-500": "199 89% 48%",
      "--brand-600": "199 89% 38%",
      "--brand-700": "199 89% 28%",
      "--brand-800": "199 89% 18%",
      "--brand-900": "199 89% 12%",
      "--success": "142 76.2% 36.3%",
      "--success-50": "138 62.8% 94.6%",
      "--success-100": "134 60.3% 84%",
      "--success-500": "142 76.2% 36.3%",
      "--success-600": "142 71% 45%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "53.8 91.8% 94.6%",
      "--warning-100": "49.8 91.7% 84%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 84.2% 60.2%",
      "--danger-50": "0 85.7% 97.3%",
      "--danger-100": "0 74.7% 84%",
      "--danger-500": "0 84.2% 60.2%",
      "--danger-600": "0 72% 51%",
      "--info": "199 89% 48%",
      "--info-50": "204 100% 97%",
      "--info-100": "204 94% 94%",
      "--info-500": "199 89% 48%",
      "--info-600": "199 89% 38%",
      "--bg-primary": "0 0% 100%",
      "--bg-secondary": "204 100% 97%",
      "--bg-tertiary": "204 94% 94%",
      "--bg-overlay": "0 0% 0% / 0.5",
      "--text-primary": "222.2 84% 4.9%",
      "--text-secondary": "215.4 16.3% 46.9%",
      "--text-tertiary": "215 13.8% 34.1%",
      "--text-inverse": "0 0% 100%",
      "--radius": "12px"
    },
    dark: {
      "--background": "222 84% 4.9%",
      "--card": "222 84% 4.9%",
      "--card-foreground": "210 40% 98%",
      "--popover": "222 84% 4.9%",
      "--popover-foreground": "210 40% 98%",
      "--primary": "199 89% 48%",
      "--primary-foreground": "222 84% 4.9%",
      "--secondary": "217.2 32.6% 17.5%",
      "--secondary-foreground": "210 40% 98%",
      "--muted": "217.2 32.6% 17.5%",
      "--muted-foreground": "215 20.2% 65.1%",
      "--accent": "217.2 32.6% 17.5%",
      "--accent-foreground": "210 40% 98%",
      "--destructive": "0 62.8% 30.6%",
      "--destructive-foreground": "210 40% 98%",
      "--border": "217.2 32.6% 17.5%",
      "--input": "217.2 32.6% 17.5%",
      "--ring": "199 89% 48%",
      "--brand": "199 89% 48%",
      "--brand-50": "210 100% 8%",
      "--brand-100": "210 50% 12%",
      "--brand-200": "210 40% 16%",
      "--brand-300": "210 30% 20%",
      "--brand-400": "210 20% 24%",
      "--brand-500": "199 89% 48%",
      "--brand-600": "199 89% 38%",
      "--brand-700": "199 89% 28%",
      "--brand-800": "199 89% 18%",
      "--brand-900": "199 89% 12%",
      "--success": "142 76.2% 36.3%",
      "--success-50": "125 100% 8%",
      "--success-100": "125 80% 12%",
      "--success-500": "142 76.2% 36.3%",
      "--success-600": "142 71% 45%",
      "--warning": "45.4 93.4% 47.5%",
      "--warning-50": "36 80% 8%",
      "--warning-100": "36 93% 10%",
      "--warning-500": "45.4 93.4% 47.5%",
      "--warning-600": "35.5 91.7% 52%",
      "--danger": "0 62.8% 30.6%",
      "--danger-50": "0 80% 6%",
      "--danger-100": "0 45% 10%",
      "--danger-500": "0 62.8% 30.6%",
      "--danger-600": "0 72% 51%",
      "--info": "199 89% 48%",
      "--info-50": "210 100% 8%",
      "--info-100": "210 90% 12%",
      "--info-500": "199 89% 48%",
      "--info-600": "199 89% 38%",
      "--bg-primary": "222 84% 4.9%",
      "--bg-secondary": "217.2 32.6% 17.5%",
      "--bg-tertiary": "217.2 32.6% 17.5%",
      "--bg-overlay": "0 0% 0% / 0.7",
      "--text-primary": "210 40% 98%",
      "--text-secondary": "215 20.2% 65.1%",
      "--text-tertiary": "217.2 10.6% 64.9%",
      "--text-inverse": "222 84% 4.9%",
      "--radius": "12px"
    }
  }
};
const predefinedThemes = [
  coreBrandTheme,
  professionalBlueTheme,
  forestGreenTheme,
  twilightPurpleTheme,
  oceanDeepTheme
];
function getThemeById(id) {
  return predefinedThemes.find((theme) => theme.id === id);
}
function getDefaultTheme() {
  return coreBrandTheme;
}
const useThemePaletteStore = create()(
  persist(
    (set, get) => ({
      selectedThemeId: "core-brand",
      currentTheme: getDefaultTheme(),
      mode: "system",
      resolvedMode: "light",
      // Aliases for compatibility
      get theme() {
        return get().mode;
      },
      setTheme: (mode) => {
        get().setMode(mode);
      },
      setThemePalette: (themeId) => {
        const theme = getThemeById(themeId);
        if (theme) {
          set({ selectedThemeId: themeId, currentTheme: theme });
          get().applyTheme();
        }
      },
      setMode: (mode) => {
        const resolvedMode = mode === "system" ? window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light" : mode;
        set({ mode, resolvedMode });
        get().applyTheme();
      },
      applyTheme: () => {
        const { currentTheme, resolvedMode } = get();
        const colors = currentTheme.colors[resolvedMode];
        if (!colors) return;
        const root = document.documentElement;
        root.classList.remove("light", "dark");
        root.classList.add(resolvedMode);
        Object.entries(colors).forEach(([property, value]) => {
          root.style.setProperty(property, value);
        });
      },
      init: () => {
        const { selectedThemeId } = get();
        const theme = getThemeById(selectedThemeId) || getDefaultTheme();
        set({ selectedThemeId: theme.id, currentTheme: theme });
        const { mode } = get();
        get().setMode(mode);
      },
      getResolvedColors: () => {
        const { currentTheme, resolvedMode } = get();
        return currentTheme.colors[resolvedMode] || {};
      }
    }),
    {
      name: "mockforge-theme-palette",
      partialize: (state) => ({
        selectedThemeId: state.selectedThemeId,
        mode: state.mode
      }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.init();
        }
      }
    }
  )
);
if (typeof window !== "undefined") {
  const { setMode } = useThemePaletteStore.getState();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const { mode } = useThemePaletteStore.getState();
    if (mode === "system") {
      setMode("system");
    }
  });
}
function SimpleThemeToggle({ className, size = "md" }) {
  const { theme: resolvedTheme, setTheme } = useThemePaletteStore();
  const toggleTheme = () => {
    setTheme(resolvedTheme === "dark" ? "light" : "dark");
  };
  const sizeClasses2 = {
    sm: "h-8 w-8",
    md: "h-9 w-9",
    lg: "h-10 w-10"
  };
  const iconSizes2 = {
    sm: "h-4 w-4",
    md: "h-4 w-4",
    lg: "h-5 w-5"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Button$1,
    {
      variant: "outline",
      size: "sm",
      onClick: toggleTheme,
      className: cn(
        "btn-hover transition-all duration-200",
        sizeClasses2[size],
        className
      ),
      "aria-label": `Switch to ${resolvedTheme === "light" ? "dark" : "light"} mode`,
      children: resolvedTheme === "light" ? /* @__PURE__ */ jsxRuntimeExports.jsx(Moon, { className: iconSizes2[size] }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Sun, { className: iconSizes2[size] })
    }
  );
}
class AuthApiService {
  async fetchJson(url, options) {
    const response = await fetch(url, {
      ...options,
      headers: {
        "Content-Type": "application/json",
        ...options == null ? void 0 : options.headers
      }
    });
    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ error: "Unknown error" }));
      throw new Error(errorData.error || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    if (!json.success) {
      throw new Error(json.error || "Request failed");
    }
    return json.data;
  }
  async login(username, password) {
    return this.fetchJson("/__mockforge/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password })
    });
  }
  async refreshToken(refreshToken) {
    return this.fetchJson("/__mockforge/auth/refresh", {
      method: "POST",
      body: JSON.stringify({ refresh_token: refreshToken })
    });
  }
  async logout() {
    try {
      await this.fetchJson("/__mockforge/auth/logout", {
        method: "POST"
      });
    } catch (error) {
      console.warn("Logout request failed:", error);
    }
  }
}
const authApi = new AuthApiService();
const parseToken = (token) => {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return { user: null, expiresAt: null };
    const payload = JSON.parse(atob(parts[1]));
    const expiresAt = payload.exp * 1e3;
    if (expiresAt < Date.now()) {
      return { user: null, expiresAt: null };
    }
    const user = {
      id: payload.sub,
      username: payload.username,
      email: payload.email || "",
      role: payload.role
    };
    return { user, expiresAt };
  } catch {
    return { user: null, expiresAt: null };
  }
};
let tokenRefreshInterval = null;
const useAuthStore = create()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      refreshToken: null,
      isAuthenticated: false,
      isLoading: false,
      login: async (username, password) => {
        set({ isLoading: true });
        try {
          const response = await authApi.login(username, password);
          set({
            user: response.user,
            token: response.token,
            refreshToken: response.refresh_token,
            isAuthenticated: true,
            isLoading: false
          });
          get().startTokenRefresh();
        } catch (error) {
          set({ isLoading: false });
          const errorMessage = error instanceof Error ? error.message : "Login failed";
          logger.error("Login failed", errorMessage);
          throw new Error(errorMessage);
        }
      },
      logout: async () => {
        get().stopTokenRefresh();
        try {
          await authApi.logout();
        } catch (error) {
          logger.warn("Logout API call failed", error);
        }
        set({
          user: null,
          token: null,
          refreshToken: null,
          isAuthenticated: false,
          isLoading: false
        });
      },
      refreshTokenAction: async () => {
        const { refreshToken } = get();
        if (!refreshToken) throw new Error("No refresh token available");
        try {
          const response = await authApi.refreshToken(refreshToken);
          set({
            token: response.token,
            refreshToken: response.refresh_token,
            user: response.user
            // Update user info in case it changed
          });
        } catch (error) {
          logger.error("Token refresh failed", error);
          get().logout();
          throw error;
        }
      },
      checkTokenExpiry: () => {
        const { token } = get();
        if (!token) return false;
        try {
          const { expiresAt } = parseToken(token);
          if (!expiresAt) return false;
          const timeUntilExpiry = expiresAt - Date.now();
          return timeUntilExpiry > 5 * 60 * 1e3;
        } catch {
          return false;
        }
      },
      checkAuth: async () => {
        const { token, refreshToken } = get();
        if (!token) {
          set({ isAuthenticated: false, isLoading: false });
          return;
        }
        set({ isLoading: true });
        try {
          const { user, expiresAt } = parseToken(token);
          if (user && expiresAt && expiresAt > Date.now()) {
            set({
              user,
              isAuthenticated: true,
              isLoading: false
            });
            get().startTokenRefresh();
          } else if (refreshToken) {
            try {
              await get().refreshTokenAction();
            } catch {
              get().logout();
            }
          } else {
            get().logout();
          }
        } catch (error) {
          logger.error("Auth check failed", error);
          get().logout();
        }
      },
      updateProfile: async (userData) => {
        set({ isLoading: true });
        try {
          set({
            user: userData,
            isLoading: false
          });
          if (typeof window !== "undefined") {
            localStorage.setItem("mockforge-user-profile", JSON.stringify(userData));
          }
        } catch (error) {
          set({ isLoading: false });
          const errorMessage = error instanceof Error ? error.message : "Profile update failed";
          logger.error("Profile update failed", errorMessage);
          throw new Error(errorMessage);
        }
      },
      setAuthenticated: (user, token, refreshToken) => {
        set({
          user,
          token,
          refreshToken: refreshToken || null,
          isAuthenticated: true,
          isLoading: false
        });
        get().startTokenRefresh();
      },
      startTokenRefresh: () => {
        if (tokenRefreshInterval) {
          clearInterval(tokenRefreshInterval);
        }
        tokenRefreshInterval = setInterval(async () => {
          const { token, refreshToken: refresh, isAuthenticated } = get();
          if (isAuthenticated && token && refresh) {
            try {
              const payload = JSON.parse(atob(token.split(".")[2]));
              const timeUntilExpiry = payload.exp - Math.floor(Date.now() / 1e3);
              if (timeUntilExpiry < 300) {
                await get().refreshTokenAction();
              }
            } catch {
              get().logout();
            }
          }
        }, 6e4);
      },
      stopTokenRefresh: () => {
        if (tokenRefreshInterval) {
          clearInterval(tokenRefreshInterval);
          tokenRefreshInterval = null;
        }
      }
    }),
    {
      name: "mockforge-auth",
      partialize: (state) => ({
        token: state.token,
        refreshToken: state.refreshToken,
        user: state.user,
        isAuthenticated: state.isAuthenticated
      })
    }
  )
);
const Input$1 = reactExports.forwardRef(
  ({ className, type, error, errorId, "aria-invalid": ariaInvalid, "aria-describedby": ariaDescribedby, ...props }, ref) => {
    const hasError = !!error || ariaInvalid === true || ariaInvalid === "true";
    const describedBy = [ariaDescribedby, errorId].filter(Boolean).join(" ") || void 0;
    return /* @__PURE__ */ jsxRuntimeExports.jsx(
      "input",
      {
        type,
        className: cn(
          "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          hasError && "border-red-500 focus-visible:ring-red-500",
          className
        ),
        ref,
        "aria-invalid": hasError || void 0,
        "aria-describedby": describedBy,
        ...props
      }
    );
  }
);
Input$1.displayName = "Input";
function Dialog({ open, onOpenChange, children }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(DialogContext.Provider, { value: { open, onOpenChange }, children });
}
const DialogContext = React.createContext(null);
const useDialogContext = () => {
  const context = React.useContext(DialogContext);
  if (!context) {
    throw new Error("Dialog components must be used within a Dialog");
  }
  return context;
};
function DialogContent({ children, className }) {
  const { open, onOpenChange } = useDialogContext();
  const dialogRef = reactExports.useRef(null);
  const previouslyFocusedElement = reactExports.useRef(null);
  reactExports.useEffect(() => {
    if (!open) return;
    previouslyFocusedElement.current = document.activeElement;
    if (dialogRef.current) {
      dialogRef.current.focus();
    }
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
      if (previouslyFocusedElement.current) {
        previouslyFocusedElement.current.focus();
      }
    };
  }, [open]);
  reactExports.useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onOpenChange(false);
      }
      if (e.key === "Tab") {
        if (!dialogRef.current) return;
        const focusableElements = dialogRef.current.querySelectorAll(
          'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
        );
        const firstElement = focusableElements[0];
        const lastElement = focusableElements[focusableElements.length - 1];
        if (e.shiftKey && document.activeElement === firstElement) {
          e.preventDefault();
          lastElement == null ? void 0 : lastElement.focus();
        } else if (!e.shiftKey && document.activeElement === lastElement) {
          e.preventDefault();
          firstElement == null ? void 0 : firstElement.focus();
        }
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, onOpenChange]);
  if (!open) return null;
  const dialogContent = /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "fixed inset-0 z-50 flex items-center justify-center", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      "div",
      {
        className: "fixed inset-0 bg-black/50 backdrop-blur-sm",
        onClick: () => onOpenChange(false),
        "aria-hidden": "true"
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      "div",
      {
        ref: dialogRef,
        role: "dialog",
        "aria-modal": "true",
        tabIndex: -1,
        className: cn(
          "relative bg-bg-primary border border-border rounded-lg shadow-xl max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto focus:outline-none",
          className
        ),
        children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-6", children })
      }
    )
  ] });
  return reactDomExports.createPortal(dialogContent, document.body);
}
function DialogHeader({ children, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("flex items-center justify-between pb-4 border-b border-border", className), children });
}
function DialogTitle({ children, className, id = "dialog-title" }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { id, className: cn("text-lg font-semibold text-gray-900 dark:text-gray-100", className), children });
}
function DialogDescription({ children, className, id = "dialog-description" }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("p", { id, className: cn("text-sm text-gray-600 dark:text-gray-400 mt-1", className), children });
}
function DialogFooter({ children, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("flex items-center justify-end gap-3 pt-4 border-t border-border", className), children });
}
function DialogTrigger({ children, onClick, asChild }) {
  const { onOpenChange } = useDialogContext();
  const handleClick = () => {
    onOpenChange(true);
    onClick == null ? void 0 : onClick();
  };
  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(children, {
      ...children.props,
      onClick: handleClick
    });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { onClick: handleClick, children });
}
function DialogClose({ onClick, className }) {
  const { onOpenChange } = useDialogContext();
  const handleClick = () => {
    onOpenChange(false);
    onClick == null ? void 0 : onClick();
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Button$1,
    {
      variant: "ghost",
      size: "sm",
      className: cn("h-8 w-8 p-0 hover:bg-bg-tertiary", className),
      onClick: handleClick,
      children: /* @__PURE__ */ jsxRuntimeExports.jsx(X, { className: "h-4 w-4" })
    }
  );
}
function AccountSettings({ open, onOpenChange }) {
  const { user } = useAuthStore();
  const [formData, setFormData] = reactExports.useState({
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
    twoFactorEnabled: false,
    emailNotifications: true,
    securityAlerts: true
  });
  const [isSubmitting, setIsSubmitting] = reactExports.useState(false);
  const [errors, setErrors] = reactExports.useState({});
  const [successMessage, setSuccessMessage] = reactExports.useState("");
  React.useEffect(() => {
    if (open) {
      setFormData({
        currentPassword: "",
        newPassword: "",
        confirmPassword: "",
        twoFactorEnabled: false,
        emailNotifications: true,
        securityAlerts: true
      });
      setErrors({});
      setSuccessMessage("");
    }
  }, [open]);
  const validateForm = () => {
    const newErrors = {};
    if (formData.newPassword) {
      if (!formData.currentPassword) {
        newErrors.currentPassword = "Current password is required to set a new password";
      }
      if (formData.newPassword.length < 8) {
        newErrors.newPassword = "Password must be at least 8 characters";
      }
      if (formData.newPassword !== formData.confirmPassword) {
        newErrors.confirmPassword = "Passwords do not match";
      }
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };
  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!validateForm() || !user) return;
    setIsSubmitting(true);
    setSuccessMessage("");
    try {
      await new Promise((resolve) => setTimeout(resolve, 1e3));
      setSuccessMessage("Account settings updated successfully");
      setFormData((prev) => ({
        ...prev,
        currentPassword: "",
        newPassword: "",
        confirmPassword: ""
      }));
    } catch {
      setErrors({ general: "Failed to update account settings. Please try again." });
    } finally {
      setIsSubmitting(false);
    }
  };
  const handleInputChange = (field, value) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    if (errors[field]) {
      setErrors((prev) => ({ ...prev, [field]: "" }));
    }
    setSuccessMessage("");
  };
  if (!user) return null;
  return /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open, onOpenChange, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { className: "sm:max-w-lg bg-white dark:bg-gray-900", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { className: "space-y-2", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { className: "text-xl font-semibold text-gray-900 dark:text-gray-100", children: "Account Settings" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { className: "text-sm text-gray-600 dark:text-gray-400 leading-relaxed", children: "Manage your account security and notification preferences." }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogClose, { onClick: () => onOpenChange(false) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("form", { onSubmit: handleSubmit, className: "space-y-6", children: [
      errors.general && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-destructive bg-destructive/10 p-3 rounded-md", children: errors.general }),
      successMessage && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900/30 p-3 rounded-md", children: successMessage }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 text-sm font-semibold text-gray-900 dark:text-gray-100", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Shield, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Security" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "currentPassword", className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Current Password" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "currentPassword",
              type: "password",
              value: formData.currentPassword,
              onChange: (e) => handleInputChange("currentPassword", e.target.value),
              placeholder: "Enter current password",
              className: `bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.currentPassword ? "border-destructive" : ""}`
            }
          ),
          errors.currentPassword && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-destructive", children: errors.currentPassword })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "newPassword", className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "New Password" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "newPassword",
              type: "password",
              value: formData.newPassword,
              onChange: (e) => handleInputChange("newPassword", e.target.value),
              placeholder: "Enter new password (min 8 characters)",
              className: `bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.newPassword ? "border-destructive" : ""}`
            }
          ),
          errors.newPassword && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-destructive", children: errors.newPassword })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "confirmPassword", className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Confirm New Password" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "confirmPassword",
              type: "password",
              value: formData.confirmPassword,
              onChange: (e) => handleInputChange("confirmPassword", e.target.value),
              placeholder: "Confirm new password",
              className: `bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.confirmPassword ? "border-destructive" : ""}`
            }
          ),
          errors.confirmPassword && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-destructive", children: errors.confirmPassword })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800/50 rounded-md", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Lock, { className: "h-4 w-4 text-gray-600 dark:text-gray-400" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Two-Factor Authentication" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "input",
              {
                type: "checkbox",
                checked: formData.twoFactorEnabled,
                onChange: (e) => handleInputChange("twoFactorEnabled", e.target.checked),
                className: "sr-only peer"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600" })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 text-sm font-semibold text-gray-900 dark:text-gray-100", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Bell, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Notifications" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800/50 rounded-md", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Mail, { className: "h-4 w-4 text-gray-600 dark:text-gray-400" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Email Notifications" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "input",
              {
                type: "checkbox",
                checked: formData.emailNotifications,
                onChange: (e) => handleInputChange("emailNotifications", e.target.checked),
                className: "sr-only peer"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600" })
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800/50 rounded-md", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Shield, { className: "h-4 w-4 text-gray-600 dark:text-gray-400" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Security Alerts" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "input",
              {
                type: "checkbox",
                checked: formData.securityAlerts,
                onChange: (e) => handleInputChange("securityAlerts", e.target.checked),
                className: "sr-only peer"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600" })
          ] })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Button$1,
        {
          type: "button",
          variant: "outline",
          onClick: () => onOpenChange(false),
          disabled: isSubmitting,
          children: "Cancel"
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Button$1,
        {
          type: "submit",
          onClick: handleSubmit,
          disabled: isSubmitting,
          children: isSubmitting ? "Saving..." : "Save Changes"
        }
      )
    ] })
  ] }) });
}
function ProfileSettings({ open, onOpenChange }) {
  const { user, updateProfile } = useAuthStore();
  const [formData, setFormData] = reactExports.useState({
    username: (user == null ? void 0 : user.username) || "",
    email: (user == null ? void 0 : user.email) || ""
  });
  const [isSubmitting, setIsSubmitting] = reactExports.useState(false);
  const [errors, setErrors] = reactExports.useState({});
  React.useEffect(() => {
    if (user && open) {
      setFormData({
        username: user.username,
        email: user.email || ""
      });
      setErrors({});
    }
  }, [user, open]);
  const validateForm = () => {
    const newErrors = {};
    if (!formData.username.trim()) {
      newErrors.username = "Username is required";
    } else if (formData.username.length < 3) {
      newErrors.username = "Username must be at least 3 characters";
    }
    if (formData.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(formData.email)) {
      newErrors.email = "Please enter a valid email address";
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };
  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!validateForm() || !user) return;
    setIsSubmitting(true);
    try {
      await updateProfile({
        ...user,
        username: formData.username.trim(),
        email: formData.email.trim() || user.email
      });
      onOpenChange(false);
    } catch {
      setErrors({ general: "Failed to update profile. Please try again." });
    } finally {
      setIsSubmitting(false);
    }
  };
  const handleInputChange = (field, value) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    if (errors[field]) {
      setErrors((prev) => ({ ...prev, [field]: "" }));
    }
  };
  if (!user) return null;
  return /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open, onOpenChange, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { className: "sm:max-w-md bg-white dark:bg-gray-900", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { className: "space-y-2", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { className: "text-xl font-semibold text-gray-900 dark:text-gray-100", children: "Profile Settings" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { className: "text-sm text-gray-600 dark:text-gray-400 leading-relaxed", children: "Update your account information and preferences." }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogClose, { onClick: () => onOpenChange(false) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("form", { onSubmit: handleSubmit, className: "space-y-4", children: [
      errors.general && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-destructive bg-destructive/10 p-3 rounded-md", children: errors.general }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "username", className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Username" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Input$1,
          {
            id: "username",
            type: "text",
            value: formData.username,
            onChange: (e) => handleInputChange("username", e.target.value),
            placeholder: "Enter your username",
            className: `bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.username ? "border-destructive" : ""}`
          }
        ),
        errors.username && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-destructive", children: errors.username })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "email", className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Email (Optional)" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Input$1,
          {
            id: "email",
            type: "email",
            value: formData.email,
            onChange: (e) => handleInputChange("email", e.target.value),
            placeholder: "Enter your email address",
            className: `bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.email ? "border-destructive" : ""}`
          }
        ),
        errors.email && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-destructive", children: errors.email })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Role" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: `text-xs px-2 py-1 rounded-full ${user.role === "admin" ? "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200" : "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200"}`, children: [
            user.role === "admin" ? "🔑" : "👁️",
            " ",
            user.role
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm text-gray-600 dark:text-gray-400", children: "(Contact administrator to change role)" })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Button$1,
        {
          type: "button",
          variant: "outline",
          onClick: () => onOpenChange(false),
          disabled: isSubmitting,
          children: "Cancel"
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Button$1,
        {
          type: "submit",
          onClick: handleSubmit,
          disabled: isSubmitting,
          children: isSubmitting ? "Saving..." : "Save Changes"
        }
      )
    ] })
  ] }) });
}
const Switch = reactExports.forwardRef(({ className, ...props }, ref) => /* @__PURE__ */ jsxRuntimeExports.jsx(
  Root,
  {
    className: cn(
      "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input",
      className
    ),
    ...props,
    ref,
    children: /* @__PURE__ */ jsxRuntimeExports.jsx(
      Thumb,
      {
        className: cn(
          "pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0"
        )
      }
    )
  }
));
Switch.displayName = Root.displayName;
function Tabs({ value, defaultValue, onValueChange, children, className }) {
  const [internalValue, setInternalValue] = React.useState(defaultValue || value || "");
  const currentValue = value || internalValue;
  const handleValueChange = (newValue) => {
    setInternalValue(newValue);
    onValueChange == null ? void 0 : onValueChange(newValue);
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(TabsProvider, { value: currentValue, onValueChange: handleValueChange, children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("w-full", className), children }) });
}
function TabsList({ children, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: cn(
        "inline-flex h-10 items-center justify-center rounded-md bg-bg-tertiary p-1 text-secondary",
        className
      ),
      children
    }
  );
}
function TabsTrigger({ value, children, className }) {
  const context = React.useContext(TabsContext);
  const isActive = value === (context == null ? void 0 : context.activeTab);
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "button",
    {
      className: cn(
        "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-bg-primary transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
        isActive ? "bg-bg-primary text-primary shadow-sm" : "text-secondary hover:text-primary",
        className
      ),
      onClick: () => context == null ? void 0 : context.onTabChange(value),
      children
    }
  );
}
function TabsContent({ value, children, className }) {
  var _a;
  const isActive = value === ((_a = React.useContext(TabsContext)) == null ? void 0 : _a.activeTab);
  if (!isActive) return null;
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("mt-2 ring-offset-bg-primary focus-visible:outline-none", className), children });
}
const TabsContext = React.createContext(null);
function TabsProvider({
  value,
  onValueChange,
  children
}) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContext.Provider, { value: { activeTab: value, onTabChange: onValueChange }, children });
}
const defaultThemePreferences = {
  theme: "system",
  accentColor: "blue",
  fontSize: "medium",
  highContrast: false
};
const defaultLogPreferences = {
  autoScroll: true,
  pauseOnError: false,
  defaultTimeRange: 24,
  itemsPerPage: 100,
  showTimestamps: true,
  compactView: false
};
const defaultNotificationPreferences = {
  enableSounds: false,
  showToasts: true,
  toastDuration: 5,
  notifyOnErrors: true,
  notifyOnSuccess: false
};
const defaultSearchPreferences = {
  defaultScope: "all",
  searchHistory: [],
  maxHistoryItems: 10,
  caseSensitive: false,
  regexEnabled: false
};
const defaultUIBehaviorPreferences = {
  sidebarCollapsed: false,
  defaultPage: "dashboard",
  confirmDelete: true,
  autoSave: true,
  keyboardShortcuts: true,
  serverTableDensity: "comfortable"
};
const defaultPreferences = {
  theme: defaultThemePreferences,
  logs: defaultLogPreferences,
  notifications: defaultNotificationPreferences,
  search: defaultSearchPreferences,
  ui: defaultUIBehaviorPreferences
};
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const usePreferencesStore = create()(
  persist(
    (set, get) => ({
      preferences: defaultPreferences,
      loading: false,
      error: null,
      updatePreferences: (newPreferences) => {
        const current = get().preferences;
        set({
          preferences: { ...current, ...newPreferences },
          error: null
        });
      },
      updateTheme: (themeUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            theme: { ...current.theme, ...themeUpdates }
          },
          error: null
        });
      },
      updateLogs: (logsUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            logs: { ...current.logs, ...logsUpdates }
          },
          error: null
        });
      },
      updateNotifications: (notificationUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            notifications: { ...current.notifications, ...notificationUpdates }
          },
          error: null
        });
      },
      updateSearch: (searchUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            search: { ...current.search, ...searchUpdates }
          },
          error: null
        });
      },
      updateUI: (uiUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            ui: { ...current.ui, ...uiUpdates }
          },
          error: null
        });
      },
      loadPreferences: async () => {
        set({ loading: true, error: null });
        try {
          await delay(800);
          set({ loading: false });
        } catch (error) {
          set({
            loading: false,
            error: error instanceof Error ? error.message : "Failed to load preferences"
          });
          throw error;
        }
      },
      resetToDefaults: () => {
        set({
          preferences: defaultPreferences,
          error: null
        });
      },
      savePreferences: async () => {
        set({ loading: true, error: null });
        try {
          await delay(800);
          set({ loading: false });
        } catch (error) {
          set({
            loading: false,
            error: error instanceof Error ? error.message : "Failed to save preferences"
          });
          throw error;
        }
      }
    }),
    {
      name: "mockforge-preferences",
      partialize: (state) => ({
        preferences: state.preferences
      })
    }
  )
);
const useThemeStore = create()(
  persist(
    (set, get) => ({
      theme: "system",
      resolvedTheme: "light",
      setTheme: (theme) => {
        const resolvedTheme = theme === "system" ? window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light" : theme;
        set({ theme, resolvedTheme });
        const root = document.documentElement;
        root.classList.remove("light", "dark");
        root.classList.add(resolvedTheme);
      },
      toggleTheme: () => {
        const currentResolved = get().resolvedTheme;
        const newTheme = currentResolved === "light" ? "dark" : "light";
        get().setTheme(newTheme);
      },
      // Initialize theme on load
      init: () => {
        const { theme } = get();
        get().setTheme(theme);
      }
    }),
    {
      name: "mockforge-theme",
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.init();
        }
      }
    }
  )
);
if (typeof window !== "undefined") {
  const store = useThemeStore.getState();
  store.init();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const { theme } = useThemeStore.getState();
    if (theme === "system") {
      useThemeStore.getState().setTheme("system");
    }
  });
}
function Preferences({ open, onOpenChange }) {
  const [activeTab, setActiveTab] = reactExports.useState("theme");
  const {
    preferences,
    updateTheme,
    updateLogs,
    updateNotifications,
    updateSearch,
    updateUI,
    resetToDefaults,
    savePreferences,
    loading,
    error
  } = usePreferencesStore();
  const { setTheme: setThemeStore } = useThemeStore();
  const handleSave = async () => {
    try {
      await savePreferences();
      onOpenChange(false);
    } catch (error2) {
      logger.error("Failed to save preferences", error2);
    }
  };
  const handleReset = () => {
    resetToDefaults();
    setThemeStore("system");
  };
  const themeOptions = [
    { value: "light", label: "Light", icon: Sun },
    { value: "dark", label: "Dark", icon: Moon },
    { value: "system", label: "System", icon: Monitor }
  ];
  const accentColors = [
    { value: "blue", label: "Blue" },
    { value: "green", label: "Green" },
    { value: "purple", label: "Purple" },
    { value: "orange", label: "Orange" },
    { value: "red", label: "Red" }
  ];
  return /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open, onOpenChange, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { className: "sm:max-w-4xl max-h-[90vh] overflow-y-auto bg-white dark:bg-gray-900", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { className: "space-y-2", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogTitle, { className: "flex items-center gap-2 text-xl font-semibold text-gray-900 dark:text-gray-100", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "h-5 w-5" }),
        "Preferences"
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { className: "text-sm text-gray-600 dark:text-gray-400 leading-relaxed", children: "Customize your experience with MockForge" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogClose, { onClick: () => onOpenChange(false) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(TabsProvider, { value: activeTab, onValueChange: setActiveTab, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Tabs, { value: activeTab, onValueChange: setActiveTab, className: "w-full", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsList, { className: "grid w-full grid-cols-5 bg-gray-100 dark:bg-gray-800", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "theme", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Palette, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "Theme" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "logs", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(FileText, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "Logs" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "notifications", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Bell, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "Notifications" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "search", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Search, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "Search" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "ui", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "UI" })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "theme", className: "space-y-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 block", children: "Theme Mode" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid grid-cols-3 gap-2", children: themeOptions.map(({ value, label, icon: Icon2 }) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
            "button",
            {
              onClick: () => {
                updateTheme({ theme: value });
                setThemeStore(value);
              },
              className: `flex items-center gap-2 p-3 rounded-lg border transition-all ${preferences.theme.theme === value ? "border-orange-500 bg-orange-50 dark:bg-orange-900/20 text-orange-700 dark:text-orange-300" : "border-gray-300 dark:border-gray-600 hover:border-orange-300 dark:hover:border-orange-600 text-gray-700 dark:text-gray-300"}`,
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: "h-4 w-4" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm", children: label })
              ]
            },
            value
          )) })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 block", children: "Accent Color" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex gap-2", children: accentColors.map(({ value, label }) => /* @__PURE__ */ jsxRuntimeExports.jsx(
            "button",
            {
              onClick: () => updateTheme({ accentColor: value }),
              className: `w-8 h-8 rounded-full border-2 transition-all ${preferences.theme.accentColor === value ? "border-gray-900 dark:border-gray-100 scale-110 shadow-lg" : "border-gray-300 dark:border-gray-600 hover:scale-105"}`,
              style: { backgroundColor: value },
              title: label
            },
            value
          )) })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "High Contrast" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Increase contrast for better accessibility" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.theme.highContrast,
              onCheckedChange: (checked) => updateTheme({ highContrast: checked })
            }
          )
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "logs", className: "space-y-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Auto-scroll" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Automatically scroll to new log entries" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.logs.autoScroll,
              onCheckedChange: (checked) => updateLogs({ autoScroll: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Pause on Error" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Pause log streaming when errors occur" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.logs.pauseOnError,
              onCheckedChange: (checked) => updateLogs({ pauseOnError: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Show Timestamps" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Display timestamps in log entries" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.logs.showTimestamps,
              onCheckedChange: (checked) => updateLogs({ showTimestamps: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Compact View" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Use compact layout for log entries" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.logs.compactView,
              onCheckedChange: (checked) => updateLogs({ compactView: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block", children: "Default Time Range (hours)" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              type: "number",
              min: "1",
              max: "168",
              value: preferences.logs.defaultTimeRange,
              onChange: (e) => updateLogs({ defaultTimeRange: parseInt(e.target.value) || 24 }),
              className: "w-24 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block", children: "Items Per Page" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              type: "number",
              min: "10",
              max: "1000",
              step: "10",
              value: preferences.logs.itemsPerPage,
              onChange: (e) => updateLogs({ itemsPerPage: parseInt(e.target.value) || 100 }),
              className: "w-24 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
            }
          )
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "notifications", className: "space-y-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Enable Sounds" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Play notification sounds" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.notifications.enableSounds,
              onCheckedChange: (checked) => updateNotifications({ enableSounds: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Show Toasts" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Display toast notifications" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.notifications.showToasts,
              onCheckedChange: (checked) => updateNotifications({ showToasts: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Notify on Errors" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Show notifications for error events" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.notifications.notifyOnErrors,
              onCheckedChange: (checked) => updateNotifications({ notifyOnErrors: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Notify on Success" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Show notifications for successful operations" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.notifications.notifyOnSuccess,
              onCheckedChange: (checked) => updateNotifications({ notifyOnSuccess: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block", children: "Toast Duration (seconds)" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              type: "number",
              min: "1",
              max: "30",
              value: preferences.notifications.toastDuration,
              onChange: (e) => updateNotifications({ toastDuration: parseInt(e.target.value) || 5 }),
              className: "w-24 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
            }
          )
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "search", className: "space-y-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 block", children: "Default Search Scope" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid grid-cols-2 gap-2", children: [
            { value: "all", label: "All" },
            { value: "current", label: "Current Page" },
            { value: "logs", label: "Logs Only" },
            { value: "services", label: "Services Only" }
          ].map(({ value, label }) => /* @__PURE__ */ jsxRuntimeExports.jsx(
            "button",
            {
              onClick: () => updateSearch({ defaultScope: value }),
              className: `p-2 text-sm rounded border transition-all ${preferences.search.defaultScope === value ? "border-brand bg-brand/10 text-brand" : "border-border hover:border-brand/50"}`,
              children: label
            },
            value
          )) })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Case Sensitive" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Match case in search queries" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.search.caseSensitive,
              onCheckedChange: (checked) => updateSearch({ caseSensitive: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Regex Enabled" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Allow regular expressions in search" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.search.regexEnabled,
              onCheckedChange: (checked) => updateSearch({ regexEnabled: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block", children: "Max History Items" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              type: "number",
              min: "5",
              max: "50",
              value: preferences.search.maxHistoryItems,
              onChange: (e) => updateSearch({ maxHistoryItems: parseInt(e.target.value) || 10 }),
              className: "w-24 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400"
            }
          )
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "ui", className: "space-y-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Sidebar Collapsed" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Start with collapsed sidebar" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.ui.sidebarCollapsed,
              onCheckedChange: (checked) => updateUI({ sidebarCollapsed: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Confirm Delete" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Show confirmation dialogs for delete actions" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.ui.confirmDelete,
              onCheckedChange: (checked) => updateUI({ confirmDelete: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Auto-save" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Automatically save changes" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.ui.autoSave,
              onCheckedChange: (checked) => updateUI({ autoSave: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Keyboard Shortcuts" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-600 dark:text-gray-400", children: "Enable keyboard shortcuts" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Switch,
            {
              checked: preferences.ui.keyboardShortcuts,
              onCheckedChange: (checked) => updateUI({ keyboardShortcuts: checked })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block", children: "Default Page" }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            "select",
            {
              value: preferences.ui.defaultPage,
              onChange: (e) => updateUI({ defaultPage: e.target.value }),
              className: "w-full p-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "dashboard", children: "Dashboard" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "services", children: "Services" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "logs", children: "Logs" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "fixtures", children: "Fixtures" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "metrics", children: "Metrics" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "testing", children: "Testing" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "config", children: "Config" })
              ]
            }
          )
        ] })
      ] }) })
    ] }) }),
    error && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-destructive bg-destructive/10 p-3 rounded-md mt-4", children: error }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          type: "button",
          variant: "outline",
          onClick: handleReset,
          className: "flex items-center gap-2",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(RotateCcw, { className: "h-4 w-4" }),
            "Reset to Defaults"
          ]
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            type: "button",
            variant: "outline",
            onClick: () => onOpenChange(false),
            disabled: loading,
            children: "Cancel"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            type: "button",
            onClick: handleSave,
            disabled: loading,
            children: loading ? "Saving..." : "Save Preferences"
          }
        )
      ] })
    ] })
  ] }) });
}
function HelpSupport({ open, onOpenChange }) {
  const [activeTab, setActiveTab] = reactExports.useState("quickstart");
  const isMac = navigator.userAgent.toUpperCase().indexOf("MAC") >= 0;
  const modKey = isMac ? "⌘" : "Ctrl";
  const shortcuts = [
    { keys: `${modKey} + K`, description: "Focus global search" },
    { keys: "Esc", description: "Clear search / Close dialogs" },
    { keys: `${modKey} + /`, description: "Show keyboard shortcuts" }
  ];
  const faqs = [
    {
      question: "How do I create a new workspace?",
      answer: 'Navigate to the Workspaces page and click the "New Workspace" button. Fill in the required details and click "Create".'
    },
    {
      question: "How do I import fixtures from OpenAPI/Swagger?",
      answer: 'Go to the Import page, select "OpenAPI/Swagger" as the source, upload your spec file or provide a URL, and click "Import".'
    },
    {
      question: "What are chains and how do I use them?",
      answer: "Chains allow you to link multiple mock responses together in sequence. Create a chain in the Chains page and define the order of responses."
    },
    {
      question: "How do I view real-time logs?",
      answer: "Visit the Logs page where you can see live request/response logs. Use filters to narrow down specific requests or services."
    },
    {
      question: "Can I export my workspace configuration?",
      answer: "Yes! Go to the Workspaces page, select your workspace, and use the export option to download your configuration."
    }
  ];
  return /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open, onOpenChange, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { className: "sm:max-w-3xl max-h-[90vh] overflow-y-auto bg-white dark:bg-gray-900", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { className: "space-y-2", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogTitle, { className: "flex items-center gap-2 text-xl font-semibold text-gray-900 dark:text-gray-100", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(CircleQuestionMark, { className: "h-5 w-5" }),
        "Help & Support"
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { className: "text-sm text-gray-600 dark:text-gray-400 leading-relaxed", children: "Learn how to use MockForge effectively" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogClose, { onClick: () => onOpenChange(false) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(TabsProvider, { value: activeTab, onValueChange: setActiveTab, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Tabs, { value: activeTab, onValueChange: setActiveTab, className: "w-full", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsList, { className: "grid w-full grid-cols-3 bg-gray-100 dark:bg-gray-800", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "quickstart", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Rocket, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "Quick Start" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "shortcuts", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Keyboard, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "Shortcuts" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "faq", className: "flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(MessageCircle, { className: "h-4 w-4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: "FAQ" })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "quickstart", className: "space-y-4 mt-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3", children: "Welcome to MockForge!" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mb-4", children: "MockForge is a powerful API mocking and testing platform. Here's how to get started:" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold", children: "1" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100 mb-1", children: "Create a Workspace" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: 'Start by creating a workspace to organize your mocks. Navigate to Workspaces and click "New Workspace".' })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold", children: "2" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100 mb-1", children: "Import or Create Fixtures" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Import fixtures from OpenAPI/Swagger specs or create them manually in the Fixtures page." })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold", children: "3" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100 mb-1", children: "Configure Services" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Set up your mock services with specific routes, responses, and behaviors in the Services page." })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold", children: "4" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100 mb-1", children: "Monitor & Test" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Use the Logs and Metrics pages to monitor requests and the Testing page to validate your mocks." })
            ] })
          ] })
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "shortcuts", className: "space-y-4 mt-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3", children: "Keyboard Shortcuts" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mb-4", children: "Use these shortcuts to navigate faster:" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2", children: shortcuts.map((shortcut, index) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "div",
          {
            className: "flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800 rounded-lg",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm text-gray-600 dark:text-gray-400", children: shortcut.description }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("kbd", { className: "px-3 py-1 text-sm font-mono bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-sm text-gray-900 dark:text-gray-100", children: shortcut.keys })
            ]
          },
          index
        )) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg", children: /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-blue-800 dark:text-blue-200", children: "💡 Tip: You can enable/disable keyboard shortcuts in Preferences" }) })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "faq", className: "space-y-4 mt-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { children: /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3", children: "Frequently Asked Questions" }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: faqs.map((faq, index) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "div",
          {
            className: "p-4 bg-gray-50 dark:bg-gray-800 rounded-lg",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100 mb-2", children: faq.question }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: faq.answer })
            ]
          },
          index
        )) })
      ] }) })
    ] }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { className: "flex items-center justify-between border-t border-gray-200 dark:border-gray-700 pt-4 mt-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-4 text-sm", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "a",
          {
            href: "https://github.com/SaaSy-Solutions/mockforge",
            target: "_blank",
            rel: "noopener noreferrer",
            className: "flex items-center gap-1 text-gray-600 dark:text-gray-400 hover:text-orange-600 dark:hover:text-orange-400 transition-colors",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(ExternalLink, { className: "h-4 w-4" }),
              "GitHub"
            ]
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "a",
          {
            href: "https://docs.mockforge.dev/api/admin-ui-rest.html",
            target: "_blank",
            rel: "noopener noreferrer",
            className: "flex items-center gap-1 text-gray-600 dark:text-gray-400 hover:text-orange-600 dark:hover:text-orange-400 transition-colors",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Book, { className: "h-4 w-4" }),
              "API Docs"
            ]
          }
        )
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Button$1,
        {
          type: "button",
          onClick: () => onOpenChange(false),
          children: "Close"
        }
      )
    ] })
  ] }) });
}
function UserProfile() {
  const { user, logout } = useAuthStore();
  const [showDropdown, setShowDropdown] = reactExports.useState(false);
  const [showAccountSettings, setShowAccountSettings] = reactExports.useState(false);
  const [showProfileSettings, setShowProfileSettings] = reactExports.useState(false);
  const [showPreferences, setShowPreferences] = reactExports.useState(false);
  const [showHelpSupport, setShowHelpSupport] = reactExports.useState(false);
  if (!user) return null;
  const getRoleColor = (role) => {
    switch (role) {
      case "admin":
        return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300";
      case "viewer":
        return "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300";
      default:
        return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300";
    }
  };
  const getRoleIcon = (role) => {
    switch (role) {
      case "admin":
        return "🔑";
      case "viewer":
        return "👁️";
      default:
        return "👤";
    }
  };
  const handleLogout = () => {
    logout();
    setShowDropdown(false);
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs(
      "button",
      {
        onClick: () => setShowDropdown(!showDropdown),
        className: "flex items-center space-x-2 px-3 py-2 rounded-md hover:bg-accent transition-colors",
        children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-8 h-8 bg-primary rounded-full flex items-center justify-center text-gray-900 dark:text-gray-100-foreground text-sm font-medium", children: user.username.charAt(0).toUpperCase() }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-left", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm font-medium", children: user.username }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: `text-xs px-2 py-0.5 rounded-full inline-flex items-center space-x-1 ${getRoleColor(user.role)}`, children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: getRoleIcon(user.role) }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: user.role })
              ] })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-muted-foreground", children: showDropdown ? "▲" : "▼" })
        ]
      }
    ),
    showDropdown && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        "div",
        {
          className: "fixed inset-0 z-10",
          onClick: () => setShowDropdown(false)
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "absolute right-0 mt-2 w-64 bg-card border rounded-md shadow-lg z-20", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-4 border-b", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-10 h-10 bg-primary rounded-full flex items-center justify-center text-gray-900 dark:text-gray-100-foreground font-medium", children: user.username.charAt(0).toUpperCase() }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: user.username }),
            user.email && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: user.email }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: `text-xs px-2 py-0.5 rounded-full inline-flex items-center space-x-1 mt-1 ${getRoleColor(user.role)}`, children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: getRoleIcon(user.role) }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "capitalize", children: user.role })
            ] })
          ] })
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-1", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "px-3 py-2 text-xs text-muted-foreground", children: "Account" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "button",
              {
                className: "w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors",
                onClick: () => {
                  setShowDropdown(false);
                  setShowAccountSettings(true);
                },
                children: "Account Settings"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "button",
              {
                className: "w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors",
                onClick: () => {
                  setShowDropdown(false);
                  setShowProfileSettings(true);
                },
                children: "Profile Settings"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "button",
              {
                className: "w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors",
                onClick: () => {
                  setShowDropdown(false);
                  setShowPreferences(true);
                },
                children: "Preferences"
              }
            )
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "border-t my-2" }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-1", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "px-3 py-2 text-xs text-muted-foreground", children: "System" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "a",
              {
                href: "https://docs.mockforge.dev/api/admin-ui-rest.html",
                target: "_blank",
                rel: "noopener noreferrer",
                className: "w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors block",
                onClick: () => setShowDropdown(false),
                children: "API Documentation"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "button",
              {
                className: "w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors",
                onClick: () => {
                  setShowDropdown(false);
                  setShowHelpSupport(true);
                },
                children: "Help & Support"
              }
            )
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "border-t my-2" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Button$1,
            {
              variant: "ghost",
              className: "w-full justify-start text-destructive hover:text-destructive hover:bg-destructive/10",
              onClick: handleLogout,
              children: "Sign Out"
            }
          )
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      AccountSettings,
      {
        open: showAccountSettings,
        onOpenChange: setShowAccountSettings
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      ProfileSettings,
      {
        open: showProfileSettings,
        onOpenChange: setShowProfileSettings
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      Preferences,
      {
        open: showPreferences,
        onOpenChange: setShowPreferences
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      HelpSupport,
      {
        open: showHelpSupport,
        onOpenChange: setShowHelpSupport
      }
    )
  ] });
}
const sizeClasses = {
  sm: "h-6 w-auto",
  md: "h-8 w-auto",
  lg: "h-10 w-auto",
  xl: "h-12 w-auto"
};
function Logo({ variant = "full", size = "md", className = "", loading = "eager" }) {
  const [imageError, setImageError] = reactExports.useState(false);
  const getLogoSrc = () => {
    if (variant === "icon") {
      switch (size) {
        case "sm":
          return "/mockforge-icon-32.png";
        // 32px for sm size (optimized)
        case "md":
          return "/mockforge-icon-32.png";
        // 32px for md size (optimized)
        case "lg":
          return "/mockforge-icon-48.png";
        // 48px for lg size (optimized)
        case "xl":
          return "/mockforge-icon-48.png";
        // 48px for xl size (optimized)
        default:
          return "/mockforge-icon-48.png";
      }
    } else {
      switch (size) {
        case "sm":
          return "/mockforge-logo-40.png";
        // 40px for sm size (optimized)
        case "md":
          return "/mockforge-logo-40.png";
        // 40px for md size (optimized)
        case "lg":
          return "/mockforge-logo-40.png";
        // 40px height for lg size (optimized)
        case "xl":
          return "/mockforge-logo-80.png";
        // 80px height for xl size (optimized)
        default:
          return "/mockforge-logo-80.png";
      }
    }
  };
  const logoSrc = getLogoSrc();
  const altText = variant === "icon" ? "MockForge" : "MockForge Logo";
  const handleImageError = () => {
    logger.warn(`Failed to load logo image: ${logoSrc}. Using fallback.`);
    setImageError(true);
  };
  if (imageError) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: `flex items-center ${className}`, children: /* @__PURE__ */ jsxRuntimeExports.jsx(
      "div",
      {
        className: `bg-gradient-to-br from-orange-500 via-orange-600 to-red-600 rounded-lg px-3 py-1 ${sizeClasses[size]} flex items-center justify-center text-white font-bold text-sm shadow-md`,
        title: "MockForge (fallback logo)",
        children: variant === "icon" ? "M" : "MockForge"
      }
    ) });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "img",
    {
      src: logoSrc,
      alt: altText,
      className: `${sizeClasses[size]} ${className}`,
      loading,
      onError: handleImageError
    }
  );
}
const generateMockLog = (id) => {
  const methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];
  const paths = [
    "/api/users",
    "/api/users/123",
    "/api/orders",
    "/api/orders/456",
    "/api/products",
    "/api/auth/login",
    "/api/auth/logout",
    "/api/webhooks/stripe",
    "/health",
    "/metrics"
  ];
  const statusCodes = [200, 201, 204, 400, 401, 403, 404, 422, 500, 502];
  const userAgents = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "curl/7.68.0",
    "PostmanRuntime/7.28.4",
    "MockForge/1.0.0"
  ];
  const ips = ["192.168.1.100", "10.0.0.50", "172.16.0.25", "203.0.113.1"];
  const method = methods[Math.floor(Math.random() * methods.length)];
  const path = paths[Math.floor(Math.random() * paths.length)];
  const statusCode = statusCodes[Math.floor(Math.random() * statusCodes.length)];
  const responseTime = Math.floor(Math.random() * 2e3) + 10;
  const responseSize = Math.floor(Math.random() * 1e4) + 100;
  const hasError = statusCode >= 400 && Math.random() < 0.3;
  return {
    id: `req-${id}-${Date.now()}`,
    timestamp: (/* @__PURE__ */ new Date()).toISOString(),
    method,
    path,
    status_code: statusCode,
    response_time_ms: responseTime,
    client_ip: ips[Math.floor(Math.random() * ips.length)],
    user_agent: userAgents[Math.floor(Math.random() * userAgents.length)],
    headers: {
      "Content-Type": "application/json",
      "Accept": "application/json",
      "Authorization": "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
      "X-Request-ID": `req-${id}`,
      "User-Agent": userAgents[Math.floor(Math.random() * userAgents.length)]
    },
    request_size_bytes: Math.floor(Math.random() * 1e3) + 50,
    response_size_bytes: responseSize,
    error_message: hasError ? `${statusCode === 404 ? "Resource not found" : statusCode === 500 ? "Internal server error" : "Bad request"}` : void 0
  };
};
const initialLogs = Array.from({ length: 50 }, (_, i) => generateMockLog(i + 1));
const defaultFilter = {
  hours_ago: 24,
  limit: 100
};
const applyLogFilter = (logs, filter) => {
  let filtered = logs;
  if (filter.method) {
    filtered = filtered.filter((log) => log.method === filter.method);
  }
  if (filter.status_code) {
    filtered = filtered.filter((log) => log.status_code === filter.status_code);
  }
  if (filter.path_pattern) {
    const pattern = filter.path_pattern.toLowerCase();
    filtered = filtered.filter(
      (log) => log.path.toLowerCase().includes(pattern) || log.method.toLowerCase().includes(pattern) || log.error_message && log.error_message.toLowerCase().includes(pattern)
    );
  }
  if (filter.hours_ago) {
    const cutoff = /* @__PURE__ */ new Date();
    cutoff.setHours(cutoff.getHours() - filter.hours_ago);
    filtered = filtered.filter((log) => new Date(log.timestamp) >= cutoff);
  }
  if (filter.limit) {
    filtered = filtered.slice(-filter.limit);
  }
  return filtered.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
};
let logStreamInterval = null;
let logCounter = 51;
const useLogStore = create((set, get) => ({
  logs: initialLogs,
  filteredLogs: applyLogFilter(initialLogs, defaultFilter),
  selectedLog: null,
  filter: defaultFilter,
  autoScroll: true,
  isPaused: false,
  connectionStatus: "connected",
  setLogs: (logs) => {
    const filteredLogs = applyLogFilter(logs, get().filter);
    set({ logs, filteredLogs });
  },
  addLog: (log) => {
    const state = get();
    if (state.isPaused) return;
    const newLogs = [...state.logs, log];
    const filteredLogs = applyLogFilter(newLogs, state.filter);
    set({ logs: newLogs, filteredLogs });
  },
  selectLog: (log) => set({ selectedLog: log }),
  setFilter: (newFilter) => {
    const state = get();
    const updatedFilter = { ...state.filter, ...newFilter };
    const filteredLogs = applyLogFilter(state.logs, updatedFilter);
    set({ filter: updatedFilter, filteredLogs });
  },
  clearFilter: () => {
    const state = get();
    const clearedFilter = { hours_ago: 24, limit: 100 };
    const filteredLogs = applyLogFilter(state.logs, clearedFilter);
    set({ filter: clearedFilter, filteredLogs });
  },
  setAutoScroll: (enabled) => set({ autoScroll: enabled }),
  setPaused: (paused) => set({ isPaused: paused }),
  setConnectionStatus: (status) => set({ connectionStatus: status }),
  applyFilter: () => {
    const state = get();
    const filteredLogs = applyLogFilter(state.logs, state.filter);
    set({ filteredLogs });
  },
  clearLogs: () => set({ logs: [], filteredLogs: [], selectedLog: null }),
  startLogStream: () => {
    if (logStreamInterval) {
      clearInterval(logStreamInterval);
    }
    logStreamInterval = setInterval(() => {
      const store = get();
      if (!store.isPaused && store.connectionStatus === "connected") {
        store.addLog(generateMockLog(logCounter++));
      }
    }, 2e3 + Math.random() * 3e3);
  },
  stopLogStream: () => {
    if (logStreamInterval) {
      clearInterval(logStreamInterval);
      logStreamInterval = null;
    }
  }
}));
const originalFetch = globalThis.fetch;
function createAuthenticatedFetch() {
  return async (input, init) => {
    const state = useAuthStore.getState();
    const token = state.token;
    const headers = new Headers(init == null ? void 0 : init.headers);
    if (token) {
      headers.set("Authorization", `Bearer ${token}`);
    }
    const newInit = {
      ...init,
      headers
    };
    let response = await originalFetch(input, newInit);
    if (response.status === 401 && token) {
      try {
        await state.refreshTokenAction();
        const newToken = useAuthStore.getState().token;
        if (newToken) {
          headers.set("Authorization", `Bearer ${newToken}`);
          response = await originalFetch(input, { ...newInit, headers });
        } else {
          state.logout();
        }
      } catch (error) {
        state.logout();
      }
    }
    return response;
  };
}
const authenticatedFetch = createAuthenticatedFetch();
const mockServices = [
  {
    id: "user-service",
    name: "User Service",
    baseUrl: "http://localhost:3000",
    enabled: true,
    tags: ["api", "users"],
    description: "Handles user authentication and profile management",
    createdAt: (/* @__PURE__ */ new Date()).toISOString(),
    updatedAt: (/* @__PURE__ */ new Date()).toISOString(),
    routes: [
      {
        id: "user-service-get-users",
        method: "GET",
        path: "/api/users",
        statusCode: 200,
        priority: 1,
        has_fixtures: true,
        request_count: 234,
        error_count: 2,
        latency_ms: 45,
        enabled: true,
        service_id: "user-service",
        tags: ["api", "users"]
      },
      {
        id: "user-service-post-users",
        method: "POST",
        path: "/api/users",
        statusCode: 201,
        priority: 1,
        has_fixtures: true,
        request_count: 89,
        error_count: 0,
        latency_ms: 67,
        enabled: true,
        service_id: "user-service",
        tags: ["api", "users"]
      },
      {
        id: "user-service-get-user-id",
        method: "GET",
        path: "/api/users/{id}",
        statusCode: 200,
        priority: 1,
        has_fixtures: true,
        request_count: 156,
        error_count: 1,
        latency_ms: 32,
        enabled: false,
        service_id: "user-service",
        tags: ["api", "users"]
      }
    ]
  },
  {
    id: "order-service",
    name: "Order Service",
    baseUrl: "http://localhost:3001",
    enabled: true,
    tags: ["api", "orders", "ecommerce"],
    description: "Manages orders and order processing",
    createdAt: (/* @__PURE__ */ new Date()).toISOString(),
    updatedAt: (/* @__PURE__ */ new Date()).toISOString(),
    routes: [
      {
        id: "order-service-get-orders",
        method: "GET",
        path: "/api/orders",
        statusCode: 200,
        priority: 1,
        has_fixtures: true,
        request_count: 445,
        error_count: 5,
        latency_ms: 78,
        enabled: true,
        service_id: "order-service",
        tags: ["api", "orders"]
      },
      {
        id: "order-service-post-orders",
        method: "POST",
        path: "/api/orders",
        statusCode: 201,
        priority: 1,
        has_fixtures: true,
        request_count: 123,
        error_count: 3,
        latency_ms: 234,
        enabled: true,
        service_id: "order-service",
        tags: ["api", "orders"]
      }
    ]
  },
  {
    id: "grpc-inventory",
    name: "Inventory gRPC",
    baseUrl: "grpc://localhost:50051",
    enabled: false,
    tags: ["grpc", "inventory"],
    description: "gRPC service for inventory management",
    createdAt: (/* @__PURE__ */ new Date()).toISOString(),
    updatedAt: (/* @__PURE__ */ new Date()).toISOString(),
    routes: [
      {
        id: "grpc-inventory-get-item",
        method: "GRPC",
        path: "inventory.InventoryService/GetItem",
        statusCode: 0,
        priority: 1,
        has_fixtures: false,
        request_count: 67,
        error_count: 0,
        latency_ms: 23,
        enabled: false,
        service_id: "grpc-inventory",
        tags: ["grpc", "inventory"]
      },
      {
        id: "grpc-inventory-update-stock",
        method: "GRPC",
        path: "inventory.InventoryService/UpdateStock",
        statusCode: 0,
        priority: 1,
        has_fixtures: false,
        request_count: 34,
        error_count: 1,
        latency_ms: 56,
        enabled: false,
        service_id: "grpc-inventory",
        tags: ["grpc", "inventory"]
      }
    ]
  }
];
const filterRoutes = (services, query) => {
  const allRoutes = services.flatMap((s) => s.routes.map((r) => ({ ...r })));
  if (!query) return allRoutes;
  const q = query.toLowerCase();
  return allRoutes.filter(
    (r) => (r.method ? r.method.toLowerCase().includes(q) : false) || r.path.toLowerCase().includes(q) || r.tags && r.tags.some((t) => t.toLowerCase().includes(q))
  );
};
const SHOULD_USE_MOCK_FALLBACK = false;
const useServiceStore = create((set, _get) => ({
  services: [],
  filteredRoutes: [],
  isLoading: false,
  error: null,
  setServices: (services) => set({ services, filteredRoutes: filterRoutes(services) }),
  fetchServices: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await authenticatedFetch("/__mockforge/routes");
      if (!response.ok) {
        throw new Error(`Failed to fetch routes: ${response.statusText}`);
      }
      const routes = await response.json();
      const serviceMap = /* @__PURE__ */ new Map();
      for (const route of routes) {
        const pathParts = (route.path || "").split("/").filter(Boolean);
        const serviceName = pathParts[1] || pathParts[0] || "default";
        const serviceId = `${serviceName}-service`;
        if (!serviceMap.has(serviceId)) {
          serviceMap.set(serviceId, {
            id: serviceId,
            name: serviceName.charAt(0).toUpperCase() + serviceName.slice(1) + " Service",
            baseUrl: window.location.origin,
            enabled: true,
            tags: [],
            description: `Routes for ${serviceName}`,
            createdAt: (/* @__PURE__ */ new Date()).toISOString(),
            updatedAt: (/* @__PURE__ */ new Date()).toISOString(),
            routes: []
          });
        }
        const service = serviceMap.get(serviceId);
        service.routes.push({
          id: `${serviceId}-${route.method}-${route.path}`.replace(/[^a-zA-Z0-9-]/g, "-"),
          method: route.method || "ANY",
          path: route.path || "/",
          statusCode: route.status_code || 200,
          priority: route.priority || 1,
          has_fixtures: route.has_fixtures || false,
          request_count: route.request_count || 0,
          error_count: route.error_count || 0,
          latency_ms: route.latency_ms || 0,
          enabled: route.enabled !== false,
          service_id: serviceId,
          tags: route.tags || []
        });
      }
      const services = Array.from(serviceMap.values());
      if (services.length === 0 && SHOULD_USE_MOCK_FALLBACK) ;
      else {
        set({ services, filteredRoutes: filterRoutes(services), isLoading: false });
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Failed to fetch services";
      logger.error("Failed to fetch services", error);
      {
        set({ services: [], filteredRoutes: [], error: errorMessage, isLoading: false });
      }
    }
  },
  clearError: () => set({ error: null }),
  updateService: (serviceId, updates) => set((state) => ({
    services: state.services.map(
      (service) => service.id === serviceId ? { ...service, ...updates } : service
    )
  })),
  toggleRoute: (serviceId, routeId, enabled) => set((state) => ({
    services: state.services.map(
      (service) => service.id === serviceId ? {
        ...service,
        routes: service.routes.map((route) => {
          const id = route.method ? `${route.method}-${route.path}` : route.path;
          return id === routeId ? { ...route, enabled } : route;
        })
      } : service
    )
  })),
  addService: (service) => set((state) => ({
    services: [...state.services, service],
    filteredRoutes: filterRoutes([...state.services, service])
  })),
  removeService: (serviceId) => set((state) => ({
    services: state.services.filter((service) => service.id !== serviceId),
    filteredRoutes: filterRoutes(state.services.filter((service) => service.id !== serviceId))
  })),
  setGlobalSearch: (query) => set((state) => ({
    filteredRoutes: filterRoutes(state.services, query)
  }))
}));
function useKeyboardNavigation({
  shortcuts = [],
  element = null,
  enabled = true,
  capture = false
} = {}) {
  const [isEnabled, setIsEnabled] = reactExports.useState(enabled);
  const shortcutsRef = reactExports.useRef(shortcuts);
  reactExports.useEffect(() => {
    shortcutsRef.current = shortcuts;
  }, [shortcuts]);
  const handleKeyDown = reactExports.useCallback((event) => {
    if (!isEnabled) return;
    const keyboardEvent = event;
    const activeShortcuts = shortcutsRef.current.filter(
      (shortcut) => shortcut.enabled !== false
    );
    for (const shortcut of activeShortcuts) {
      const isMatch = keyboardEvent.key.toLowerCase() === shortcut.key.toLowerCase() && !!keyboardEvent.ctrlKey === !!shortcut.ctrl && !!keyboardEvent.shiftKey === !!shortcut.shift && !!keyboardEvent.altKey === !!shortcut.alt && !!keyboardEvent.metaKey === !!shortcut.meta;
      if (isMatch) {
        if (shortcut.preventDefault !== false) {
          keyboardEvent.preventDefault();
        }
        if (shortcut.stopPropagation) {
          keyboardEvent.stopPropagation();
        }
        shortcut.handler(keyboardEvent);
        break;
      }
    }
  }, [isEnabled]);
  reactExports.useEffect(() => {
    const targetElement = element || document;
    if (!isEnabled || !targetElement) return;
    targetElement.addEventListener("keydown", handleKeyDown, capture);
    return () => {
      targetElement.removeEventListener("keydown", handleKeyDown, capture);
    };
  }, [element, isEnabled, handleKeyDown, capture]);
  const addShortcut = reactExports.useCallback((shortcut) => {
    shortcutsRef.current = [...shortcutsRef.current, shortcut];
  }, []);
  const removeShortcut = reactExports.useCallback((key, modifiers) => {
    shortcutsRef.current = shortcutsRef.current.filter((shortcut) => {
      if (shortcut.key.toLowerCase() !== key.toLowerCase()) return true;
      if (modifiers) {
        return !(!!shortcut.ctrl === !!modifiers.ctrl && !!shortcut.shift === !!modifiers.shift && !!shortcut.alt === !!modifiers.alt && !!shortcut.meta === !!modifiers.meta);
      }
      return false;
    });
  }, []);
  const enable = reactExports.useCallback(() => setIsEnabled(true), []);
  const disable = reactExports.useCallback(() => setIsEnabled(false), []);
  const toggle = reactExports.useCallback(() => setIsEnabled((prev) => !prev), []);
  return {
    addShortcut,
    removeShortcut,
    enable,
    disable,
    toggle,
    isEnabled
  };
}
function useAppShortcuts(options = {}) {
  const shortcuts = [];
  if (options.onSearch) {
    shortcuts.push({
      key: "k",
      ctrl: true,
      handler: options.onSearch,
      description: "Search"
    });
  }
  if (options.onHelp) {
    shortcuts.push({
      key: "?",
      shift: true,
      handler: options.onHelp,
      description: "Help"
    });
  }
  if (options.onSettings) {
    shortcuts.push({
      key: ",",
      ctrl: true,
      handler: options.onSettings,
      description: "Settings"
    });
  }
  if (options.onToggleSidebar) {
    shortcuts.push({
      key: "b",
      ctrl: true,
      handler: options.onToggleSidebar,
      description: "Toggle sidebar"
    });
  }
  if (options.onNewItem) {
    shortcuts.push({
      key: "n",
      ctrl: true,
      handler: options.onNewItem,
      description: "New item"
    });
  }
  if (options.onSave) {
    shortcuts.push({
      key: "s",
      ctrl: true,
      handler: options.onSave,
      description: "Save"
    });
  }
  if (options.onUndo) {
    shortcuts.push({
      key: "z",
      ctrl: true,
      handler: options.onUndo,
      description: "Undo"
    });
  }
  if (options.onRedo) {
    shortcuts.push({
      key: "y",
      ctrl: true,
      handler: options.onRedo,
      description: "Redo"
    });
  }
  if (options.onCopy) {
    shortcuts.push({
      key: "c",
      ctrl: true,
      handler: options.onCopy,
      description: "Copy"
    });
  }
  if (options.onPaste) {
    shortcuts.push({
      key: "v",
      ctrl: true,
      handler: options.onPaste,
      description: "Paste"
    });
  }
  if (options.onCut) {
    shortcuts.push({
      key: "x",
      ctrl: true,
      handler: options.onCut,
      description: "Cut"
    });
  }
  if (options.onSelectAll) {
    shortcuts.push({
      key: "a",
      ctrl: true,
      handler: options.onSelectAll,
      description: "Select all"
    });
  }
  return useKeyboardNavigation({
    shortcuts,
    enabled: options.enabled
  });
}
function useSkipLinks() {
  const skipToContent = reactExports.useCallback((targetId) => {
    const target = document.getElementById(targetId);
    if (target) {
      target.focus();
      target.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, []);
  const createSkipLink = reactExports.useCallback((targetId, label) => {
    return {
      href: `#${targetId}`,
      onClick: (e) => {
        e.preventDefault();
        skipToContent(targetId);
      },
      onKeyDown: (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          skipToContent(targetId);
        }
      },
      children: label,
      className: "sr-only focus:not-sr-only focus:absolute focus:top-0 focus:left-0 focus:z-50 focus:p-4 focus:bg-background focus:text-foreground focus:border focus:border-border focus:rounded-md"
    };
  }, [skipToContent]);
  return {
    skipToContent,
    createSkipLink
  };
}
const en = {
  "app.brand": "MockForge",
  "app.home": "Home",
  "app.refresh": "Refresh",
  "app.loading": "Loading...",
  "app.pageNotFoundTitle": "Page Not Found",
  "app.pageNotFoundBody": "The page you're looking for doesn't exist yet.",
  "app.goToDashboard": "Go to Dashboard",
  "app.searchPlaceholder": "Global search…",
  "a11y.skipNavigation": "Skip to navigation",
  "a11y.skipMain": "Skip to main content",
  "a11y.skipSearch": "Skip to search",
  "a11y.mainNavigation": "Main navigation",
  "a11y.mainContent": "Main content",
  "nav.core": "Core",
  "nav.servicesData": "Services & Data",
  "nav.orchestration": "Orchestration",
  "nav.observability": "Observability",
  "nav.testing": "Testing",
  "nav.chaosResilience": "Chaos & Resilience",
  "nav.importTemplates": "Import & Templates",
  "nav.aiIntelligence": "AI & Intelligence",
  "nav.community": "Community",
  "nav.plugins": "Plugins",
  "nav.configuration": "Configuration",
  "tab.dashboard": "Dashboard",
  "tab.workspaces": "Workspaces",
  "tab.federation": "Federation",
  "tab.services": "Services",
  "tab.virtualBackends": "Virtual Backends",
  "tab.fixtures": "Fixtures",
  "tab.hostedMocks": "Hosted Mocks",
  "tab.tunnels": "Tunnels",
  "tab.proxyInspector": "Proxy Inspector",
  "tab.chains": "Chains",
  "tab.graph": "Graph",
  "tab.stateMachines": "State Machines",
  "tab.scenarioStudio": "Scenario Studio",
  "tab.orchestrationBuilder": "Orchestration Builder",
  "tab.orchestrationExecution": "Orchestration Execution",
  "tab.observability": "Observability",
  "tab.worldState": "World State",
  "tab.performance": "Performance",
  "tab.systemStatus": "System Status",
  "tab.incidents": "Incidents",
  "tab.logs": "Logs",
  "tab.traces": "Traces",
  "tab.metrics": "Metrics",
  "tab.analytics": "Analytics",
  "tab.pillarAnalytics": "Pillar Analytics",
  "tab.fitnessFunctions": "Fitness Functions",
  "tab.verification": "Verification",
  "tab.contractDiff": "Contract Diff",
  "tab.testing": "Testing",
  "tab.testGenerator": "Test Generator",
  "tab.testExecution": "Test Execution",
  "tab.integrationTests": "Integration Tests",
  "tab.timeTravel": "Time Travel",
  "tab.chaosEngineering": "Chaos Engineering",
  "tab.resilience": "Resilience",
  "tab.recorder": "Recorder",
  "tab.behavioralCloning": "Behavioral Cloning",
  "tab.import": "Import",
  "tab.templateMarketplace": "Template Marketplace",
  "tab.aiStudio": "AI Studio",
  "tab.mockai": "MockAI",
  "tab.mockaiOpenApiGenerator": "MockAI OpenAPI Generator",
  "tab.mockaiRules": "MockAI Rules",
  "tab.voiceLlm": "Voice + LLM",
  "tab.showcase": "Showcase",
  "tab.learningHub": "Learning Hub",
  "tab.plugins": "Plugins",
  "tab.pluginRegistry": "Plugin Registry",
  "tab.config": "Config",
  "tab.organization": "Organization",
  "tab.billing": "Billing",
  "tab.apiTokens": "API Tokens",
  "tab.byok": "BYOK Keys",
  "tab.usage": "Plan & Usage",
  "tab.userManagement": "User Management",
  "page.config.title": "Configuration",
  "page.config.subtitle": "Manage MockForge settings and preferences",
  "page.plugins.title": "Plugin Management",
  "page.plugins.subtitle": "Manage authentication, template, response, and datasource plugins",
  "page.plugins.error": "Error",
  "page.plugins.marketplaceTitle": "Plugin Marketplace",
  "page.plugins.marketplaceBody": "Browse and install plugins from the official marketplace",
  "page.plugins.browseMarketplace": "Browse Marketplace",
  "page.plugins.installPlugin": "Install Plugin",
  "page.plugins.reloadAll": "Reload All",
  "page.mockai.title": "MockAI",
  "page.mockai.description": "AI-powered mock API intelligence for realistic, context-aware responses",
  "page.mockai.quickActions": "Quick Actions",
  "page.mockai.features": "Features"
};
const es = {
  "app.brand": "MockForge",
  "app.home": "Inicio",
  "app.refresh": "Actualizar",
  "app.loading": "Cargando...",
  "app.pageNotFoundTitle": "Pagina no encontrada",
  "app.pageNotFoundBody": "La pagina que buscas aun no existe.",
  "app.goToDashboard": "Ir al panel",
  "app.searchPlaceholder": "Busqueda global…",
  "a11y.skipNavigation": "Saltar a navegacion",
  "a11y.skipMain": "Saltar al contenido principal",
  "a11y.skipSearch": "Saltar a busqueda",
  "a11y.mainNavigation": "Navegacion principal",
  "a11y.mainContent": "Contenido principal",
  "nav.core": "Nucleo",
  "nav.servicesData": "Servicios y Datos",
  "nav.orchestration": "Orquestacion",
  "nav.observability": "Observabilidad",
  "nav.testing": "Pruebas",
  "nav.chaosResilience": "Caos y Resiliencia",
  "nav.importTemplates": "Importar y Plantillas",
  "nav.aiIntelligence": "IA e Inteligencia",
  "nav.community": "Comunidad",
  "nav.plugins": "Plugins",
  "nav.configuration": "Configuracion"
};
const translations = { en, es };
const I18nContext = reactExports.createContext(void 0);
const SUPPORTED_LOCALES = ["en"];
function resolveInitialLocale() {
  const saved = localStorage.getItem("mockforge-locale");
  if (saved === "en" || saved === "es") {
    if (SUPPORTED_LOCALES.includes(saved)) {
      return saved;
    }
  }
  if (SUPPORTED_LOCALES.includes("es")) {
    const browser = navigator.language.toLowerCase();
    if (browser.startsWith("es")) {
      return "es";
    }
  }
  return "en";
}
function normalizeLocale(locale) {
  if (SUPPORTED_LOCALES.includes(locale)) {
    return locale;
  }
  return "en";
}
function I18nProvider({ children }) {
  const [locale, setLocaleState] = reactExports.useState(() => resolveInitialLocale());
  const setLocale = (next) => {
    const normalized = normalizeLocale(next);
    localStorage.setItem("mockforge-locale", normalized);
    setLocaleState(normalized);
  };
  const value = reactExports.useMemo(
    () => ({
      locale,
      supportedLocales: SUPPORTED_LOCALES,
      setLocale,
      t: (key, fallback) => translations[locale][key] ?? translations.en[key] ?? fallback ?? key
    }),
    [locale]
  );
  return /* @__PURE__ */ jsxRuntimeExports.jsx(I18nContext.Provider, { value, children });
}
function useI18n() {
  const ctx = reactExports.useContext(I18nContext);
  if (!ctx) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return ctx;
}
const stateConfig = {
  connected: {
    color: "bg-green-500",
    label: "Connected",
    icon: "wifi"
  },
  connecting: {
    color: "bg-yellow-500",
    label: "Connecting...",
    icon: "loader"
  },
  reconnecting: {
    color: "bg-yellow-500",
    label: "Reconnecting...",
    icon: "loader"
  },
  disconnected: {
    color: "bg-red-500",
    label: "Disconnected",
    icon: "wifi-off"
  }
};
function ConnectionStatus({
  state,
  className,
  showLabel = false,
  lastConnected
}) {
  const config = stateConfig[state];
  const Icon2 = config.icon === "wifi" ? Wifi : config.icon === "wifi-off" ? WifiOff : LoaderCircle;
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "div",
    {
      className: cn(
        "flex items-center gap-2",
        className
      ),
      role: "status",
      "aria-live": "polite",
      title: lastConnected ? `Last connected: ${lastConnected.toLocaleTimeString()}` : config.label,
      children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "relative flex h-2.5 w-2.5", children: [
          state === "connected" && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: cn("animate-ping absolute inline-flex h-full w-full rounded-full opacity-75", config.color) }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: cn("relative inline-flex rounded-full h-2.5 w-2.5", config.color) })
        ] }),
        showLabel && /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-xs text-gray-600 dark:text-gray-400 flex items-center gap-1", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: cn("h-3 w-3", (state === "connecting" || state === "reconnecting") && "animate-spin") }),
          config.label
        ] })
      ]
    }
  );
}
const useConnectionStore = create((set) => ({
  backendState: "connecting",
  wsState: "disconnected",
  setBackendState: (state) => set({
    backendState: state,
    lastBackendConnected: state === "connected" ? /* @__PURE__ */ new Date() : void 0
  }),
  setWsState: (state) => set({
    wsState: state,
    lastWsConnected: state === "connected" ? /* @__PURE__ */ new Date() : void 0
  })
}));
function GlobalConnectionStatus({ className }) {
  const { backendState, wsState } = useConnectionStore();
  const overallState = backendState === "disconnected" || wsState === "disconnected" ? "disconnected" : backendState === "connecting" || wsState === "connecting" ? "connecting" : backendState === "reconnecting" || wsState === "reconnecting" ? "reconnecting" : "connected";
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    ConnectionStatus,
    {
      state: overallState,
      className,
      showLabel: overallState !== "connected"
    }
  );
}
const navSections = [
  {
    titleKey: "nav.core",
    items: [
      { id: "dashboard", labelKey: "tab.dashboard", icon: ChartColumn },
      { id: "workspaces", labelKey: "tab.workspaces", icon: FolderOpen },
      { id: "federation", labelKey: "tab.federation", icon: Share2 }
    ]
  },
  {
    titleKey: "nav.servicesData",
    items: [
      { id: "services", labelKey: "tab.services", icon: Server },
      { id: "virtual-backends", labelKey: "tab.virtualBackends", icon: Database },
      { id: "fixtures", labelKey: "tab.fixtures", icon: FileJson },
      { id: "hosted-mocks", labelKey: "tab.hostedMocks", icon: Cloud },
      { id: "tunnels", labelKey: "tab.tunnels", icon: Wifi },
      { id: "proxy-inspector", labelKey: "tab.proxyInspector", icon: Search }
    ]
  },
  {
    titleKey: "nav.orchestration",
    items: [
      { id: "chains", labelKey: "tab.chains", icon: Link2 },
      { id: "graph", labelKey: "tab.graph", icon: GitBranch },
      { id: "state-machine-editor", labelKey: "tab.stateMachines", icon: GitBranch },
      { id: "scenario-studio", labelKey: "tab.scenarioStudio", icon: Film },
      { id: "orchestration-builder", labelKey: "tab.orchestrationBuilder", icon: GitBranch },
      { id: "orchestration-execution", labelKey: "tab.orchestrationExecution", icon: CirclePlay }
    ]
  },
  {
    titleKey: "nav.observability",
    items: [
      { id: "observability", labelKey: "tab.observability", icon: Eye },
      { id: "world-state", labelKey: "tab.worldState", icon: Layers },
      { id: "performance", labelKey: "tab.performance", icon: Activity },
      { id: "status", labelKey: "tab.systemStatus", icon: Globe },
      { id: "incidents", labelKey: "tab.incidents", icon: TriangleAlert },
      { id: "logs", labelKey: "tab.logs", icon: FileText },
      { id: "traces", labelKey: "tab.traces", icon: Network },
      { id: "metrics", labelKey: "tab.metrics", icon: Activity },
      { id: "analytics", labelKey: "tab.analytics", icon: ChartColumn },
      { id: "pillar-analytics", labelKey: "tab.pillarAnalytics", icon: PanelsTopLeft },
      { id: "fitness-functions", labelKey: "tab.fitnessFunctions", icon: HeartPulse },
      { id: "verification", labelKey: "tab.verification", icon: CircleCheck },
      { id: "contract-diff", labelKey: "tab.contractDiff", icon: GitCompare }
    ]
  },
  {
    titleKey: "nav.testing",
    items: [
      { id: "testing", labelKey: "tab.testing", icon: TestTube },
      { id: "test-generator", labelKey: "tab.testGenerator", icon: CodeXml },
      { id: "test-execution", labelKey: "tab.testExecution", icon: CirclePlay },
      { id: "integration-test-builder", labelKey: "tab.integrationTests", icon: Layers },
      { id: "time-travel", labelKey: "tab.timeTravel", icon: History }
    ]
  },
  {
    titleKey: "nav.chaosResilience",
    items: [
      { id: "chaos", labelKey: "tab.chaosEngineering", icon: Zap },
      { id: "resilience", labelKey: "tab.resilience", icon: Shield },
      { id: "recorder", labelKey: "tab.recorder", icon: Radio },
      { id: "behavioral-cloning", labelKey: "tab.behavioralCloning", icon: Copy }
    ]
  },
  {
    titleKey: "nav.importTemplates",
    items: [
      { id: "import", labelKey: "tab.import", icon: Import },
      { id: "template-marketplace", labelKey: "tab.templateMarketplace", icon: Store }
    ]
  },
  {
    titleKey: "nav.aiIntelligence",
    items: [
      { id: "ai-studio", labelKey: "tab.aiStudio", icon: Brain },
      { id: "mockai", labelKey: "tab.mockai", icon: Brain },
      { id: "mockai-openapi-generator", labelKey: "tab.mockaiOpenApiGenerator", icon: CodeXml },
      { id: "mockai-rules", labelKey: "tab.mockaiRules", icon: ChartColumn },
      { id: "voice", labelKey: "tab.voiceLlm", icon: Mic }
    ]
  },
  {
    titleKey: "nav.community",
    items: [
      { id: "showcase", labelKey: "tab.showcase", icon: Star },
      { id: "learning-hub", labelKey: "tab.learningHub", icon: BookOpen }
    ]
  },
  {
    titleKey: "nav.plugins",
    items: [
      { id: "plugins", labelKey: "tab.plugins", icon: Puzzle },
      { id: "plugin-registry", labelKey: "tab.pluginRegistry", icon: Package }
    ]
  },
  {
    titleKey: "nav.configuration",
    items: [
      { id: "config", labelKey: "tab.config", icon: Settings },
      { id: "organization", labelKey: "tab.organization", icon: Users },
      { id: "billing", labelKey: "tab.billing", icon: CreditCard },
      { id: "api-tokens", labelKey: "tab.apiTokens", icon: Key },
      { id: "byok", labelKey: "tab.byok", icon: Lock },
      { id: "usage", labelKey: "tab.usage", icon: ChartLine },
      { id: "user-management", labelKey: "tab.userManagement", icon: Users }
    ]
  }
];
const allNavItems = navSections.flatMap((section) => section.items);
function AppShell({ children, activeTab, onTabChange, onRefresh }) {
  var _a;
  const { t, locale, supportedLocales, setLocale } = useI18n();
  const [sidebarOpen, setSidebarOpen] = reactExports.useState(false);
  const { setFilter: setLogFilter } = useLogStore();
  const { setGlobalSearch } = useServiceStore();
  const [globalQuery, setGlobalQuery] = reactExports.useState("");
  const [isMac, setIsMac] = reactExports.useState(false);
  useAppShortcuts({
    onSearch: () => {
      const searchInput = document.getElementById("global-search-input");
      if (searchInput) {
        searchInput.focus();
        searchInput.select();
      }
    }
  });
  const { createSkipLink } = useSkipLinks();
  React.useEffect(() => {
    setIsMac(navigator.userAgent.toUpperCase().indexOf("MAC") >= 0);
  }, []);
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "min-h-screen bg-bg-secondary", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("nav", { className: "sr-only focus-within:not-sr-only", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("a", { ...createSkipLink("main-navigation", t("a11y.skipNavigation")) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("a", { ...createSkipLink("main-content", t("a11y.skipMain")) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("a", { ...createSkipLink("global-search-input", t("a11y.skipSearch")) })
    ] }),
    sidebarOpen && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "fixed inset-0 z-50 md:hidden", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        "div",
        {
          className: "fixed inset-0 bg-black/50 backdrop-blur-sm animate-fade-in",
          onClick: () => setSidebarOpen(false)
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("aside", { className: "fixed left-0 top-0 h-full w-80 max-w-[90vw] bg-background border-r border-gray-200 dark:border-gray-800 shadow-2xl animate-slide-in-left", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800 bg-card", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Logo, { variant: "icon", size: "md" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-xl font-bold text-gray-900 dark:text-gray-100", children: t("app.brand") })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Button$1,
            {
              variant: "secondary",
              size: "sm",
              onClick: () => setSidebarOpen(false),
              className: "h-10 w-10 p-0 rounded-full spring-hover",
              children: /* @__PURE__ */ jsxRuntimeExports.jsx(X, { className: "h-5 w-5" })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("nav", { className: "p-6 space-y-6 overflow-y-auto h-[calc(100%-88px)]", children: navSections.map((section, sectionIndex) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "px-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider", children: t(section.titleKey) }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-1", children: section.items.map((item, itemIndex) => {
            const Icon2 = item.icon;
            return /* @__PURE__ */ jsxRuntimeExports.jsxs(
              Button$1,
              {
                variant: activeTab === item.id ? "default" : "ghost",
                className: cn(
                  "w-full justify-start gap-4 h-10 text-sm nav-item-hover focus-ring spring-hover",
                  "animate-slide-in-up",
                  activeTab === item.id ? "bg-brand-500 text-white shadow-md hover:bg-brand-600" : "text-foreground/80 dark:text-gray-400 hover:text-foreground dark:hover:text-gray-100 hover:bg-muted/50"
                ),
                style: { animationDelay: `${(sectionIndex * 5 + itemIndex) * 20}ms` },
                onClick: () => {
                  onTabChange(item.id);
                  setSidebarOpen(false);
                },
                children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: "h-4 w-4" }),
                  t(item.labelKey)
                ]
              },
              item.id
            );
          }) })
        ] }, section.titleKey)) })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("aside", { className: "hidden md:flex md:w-64 md:flex-col md:fixed md:inset-y-0 md:z-50", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-col flex-grow bg-bg-primary border-r border-border", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3 px-6 py-4 border-b border-border flex-shrink-0", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Logo, { variant: "icon", size: "md" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-semibold text-gray-900 dark:text-gray-100", children: t("app.brand") })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("nav", { id: "main-navigation", className: "flex-1 px-4 py-6 space-y-6 overflow-y-auto", role: "navigation", "aria-label": t("a11y.mainNavigation"), children: navSections.map((section) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider", children: t(section.titleKey) }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-1", children: section.items.map((item) => {
            const Icon2 = item.icon;
            return /* @__PURE__ */ jsxRuntimeExports.jsxs(
              Button$1,
              {
                variant: activeTab === item.id ? "default" : "ghost",
                className: cn(
                  "w-full justify-start gap-3 h-9 transition-all duration-200 nav-item-hover focus-ring spring-hover",
                  activeTab === item.id ? "bg-brand-600 text-white shadow-lg ring-1 ring-brand-200/60 dark:ring-brand-600/70 hover:bg-brand-700" : "text-foreground/80 dark:text-gray-200 hover:text-foreground dark:hover:text-white hover:bg-muted/50 dark:hover:bg-white/5"
                ),
                onClick: () => onTabChange(item.id),
                children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: "h-4 w-4" }),
                  t(item.labelKey)
                ]
              },
              item.id
            );
          }) })
        ] }, section.titleKey)) })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "md:pl-64 flex flex-col flex-1 min-h-screen", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("header", { className: "sticky top-0 z-40 flex h-16 shrink-0 items-center border-b border-border bg-bg-primary shadow-sm", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "w-full max-w-[1400px] mx-auto flex items-center gap-x-4 px-4 sm:gap-x-6 sm:px-6 lg:px-8", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "ghost", size: "sm", className: "md:hidden", onClick: () => setSidebarOpen(true), children: /* @__PURE__ */ jsxRuntimeExports.jsx(Menu, { className: "h-5 w-5" }) }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3 min-w-0", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm text-gray-600 dark:text-gray-400", children: t("app.home") }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-gray-600 dark:text-gray-400", children: "/" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 truncate capitalize", children: t(((_a = allNavItems.find((n) => n.id === activeTab)) == null ? void 0 : _a.labelKey) ?? "", activeTab) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex flex-1" }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "hidden sm:flex w-72 relative items-center", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              Input$1,
              {
                placeholder: t("app.searchPlaceholder"),
                id: "global-search-input",
                value: globalQuery,
                onChange: (e) => {
                  const q = e.target.value;
                  setGlobalQuery(q);
                  setLogFilter({ path_pattern: q || void 0 });
                  setGlobalSearch(q || void 0);
                },
                onKeyDown: (e) => {
                  var _a2;
                  if (e.key === "Escape") {
                    setGlobalQuery("");
                    setLogFilter({ path_pattern: void 0 });
                    setGlobalSearch(void 0);
                    (_a2 = document.getElementById("global-search-input")) == null ? void 0 : _a2.blur();
                  }
                }
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-[10px] text-gray-600 dark:text-gray-400 border border-border rounded px-1 py-0.5 bg-bg-primary", children: isMac ? "⌘K" : "Ctrl K" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-x-4 lg:gap-x-6", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(GlobalConnectionStatus, { className: "hidden sm:flex" }),
            supportedLocales.length > 1 && /* @__PURE__ */ jsxRuntimeExports.jsx(
              "select",
              {
                value: locale,
                onChange: (e) => setLocale(e.target.value),
                className: "hidden sm:block h-9 rounded-md border border-border bg-bg-primary px-2 text-xs",
                "aria-label": "Language",
                children: supportedLocales.map((supportedLocale) => /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: supportedLocale, children: supportedLocale.toUpperCase() }, supportedLocale))
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx(SimpleThemeToggle, {}),
            /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { variant: "outline", size: "sm", onClick: onRefresh, className: "flex items-center gap-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "hidden sm:inline", children: t("app.refresh") })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(UserProfile, {})
          ] })
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("main", { id: "main-content", className: "flex-1", role: "main", "aria-label": t("a11y.mainContent"), children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full max-w-[1400px] mx-auto px-6 py-6", children }) })
      ] })
    ] })
  ] });
}
function LoginForm({ onSuccess }) {
  const [credentials, setCredentials] = reactExports.useState({
    username: "",
    password: ""
  });
  const [isLoading, setIsLoading] = reactExports.useState(false);
  const [error, setError] = reactExports.useState("");
  const { login } = useAuthStore();
  const handleSubmit = async (e) => {
    e.preventDefault();
    setIsLoading(true);
    setError("");
    try {
      await login(credentials.username, credentials.password);
      onSuccess == null ? void 0 : onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setIsLoading(false);
    }
  };
  const handleDemoLogin = (role) => {
    const demoCredentials = {
      admin: { username: "admin", password: "admin123" },
      viewer: { username: "viewer", password: "viewer123" }
    };
    setCredentials(demoCredentials[role]);
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "min-h-screen flex items-center justify-center bg-background", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "w-full max-w-md space-y-8", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex justify-center", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Logo, { variant: "full", size: "xl" }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-3xl font-bold", children: "Admin Dashboard" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "mt-2 text-muted-foreground", children: "Sign in to access the admin dashboard" })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-card border rounded-lg p-6 space-y-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("form", { onSubmit: handleSubmit, className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "username", className: "text-sm font-medium", children: "Username" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "username",
              type: "text",
              value: credentials.username,
              onChange: (e) => setCredentials((prev) => ({ ...prev, username: e.target.value })),
              placeholder: "Enter your username",
              required: true,
              autoComplete: "username"
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "password", className: "text-sm font-medium", children: "Password" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "password",
              type: "password",
              value: credentials.password,
              onChange: (e) => setCredentials((prev) => ({ ...prev, password: e.target.value })),
              placeholder: "Enter your password",
              required: true,
              autoComplete: "current-password"
            }
          )
        ] }),
        error && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded p-3", children: error }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            type: "submit",
            className: "w-full",
            disabled: isLoading || !credentials.username || !credentials.password,
            children: isLoading ? "Signing in..." : "Sign In"
          }
        )
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "absolute inset-0 flex items-center", children: /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "w-full border-t" }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "relative flex justify-center text-xs uppercase", children: /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "bg-card px-2 text-muted-foreground", children: "Demo Accounts" }) })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-2 gap-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            variant: "outline",
            onClick: () => handleDemoLogin("admin"),
            className: "w-full",
            children: "Demo Admin"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            variant: "outline",
            onClick: () => handleDemoLogin("viewer"),
            className: "w-full",
            children: "Demo Viewer"
          }
        )
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-xs text-muted-foreground text-center space-y-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Admin:" }),
          " admin / admin123 (Full access)"
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Viewer:" }),
          " viewer / viewer123 (Read-only)"
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center text-xs text-muted-foreground", children: "MockForge Admin UI v2.0 • Powered by React & Shadcn UI" })
  ] }) });
}
function AuthGuard({ children, requiredRole }) {
  const { isAuthenticated, user, isLoading, checkAuth } = useAuthStore();
  reactExports.useEffect(() => {
    checkAuth();
  }, [checkAuth]);
  if (isLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "min-h-screen flex items-center justify-center bg-background", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto", "data-testid": "loading-spinner" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-muted-foreground", children: "Checking authentication..." })
    ] }) });
  }
  if (!isAuthenticated || !user) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx(LoginForm, {});
  }
  if (requiredRole && !(user.role === "admin" || user.role === requiredRole)) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "min-h-screen flex items-center justify-center bg-background", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-6xl", children: "🔒" }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-2xl font-bold", children: "Access Denied" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mt-2", children: "You don't have permission to access this resource." }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-sm text-muted-foreground mt-1", children: [
          "Required role: ",
          requiredRole,
          " • Your role: ",
          user.role
        ] })
      ] })
    ] }) });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsx(jsxRuntimeExports.Fragment, { children });
}
function Alert({ type, variant, title, message, className, children }) {
  const alertType = type || variant || "info";
  const icons = {
    success: CircleCheckBig,
    warning: TriangleAlert,
    error: CircleAlert,
    info: Info,
    destructive: CircleAlert,
    default: Info
  };
  const colors2 = {
    success: "bg-green-50 border-green-200 text-green-800 dark:bg-green-900/20 dark:border-green-800 dark:text-green-400",
    warning: "bg-yellow-50 border-yellow-200 text-yellow-800 dark:bg-yellow-900/20 dark:border-yellow-800 dark:text-yellow-400",
    error: "bg-red-50 border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-400",
    info: "bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-900/20 dark:border-blue-800 dark:text-blue-400",
    destructive: "bg-red-50 border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-400",
    default: "bg-gray-50 border-gray-200 text-gray-800 dark:bg-gray-900/20 dark:border-gray-800 dark:text-gray-400"
  };
  const Icon2 = icons[alertType];
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn(
    "flex items-start gap-3 p-4 border rounded-xl transition-all duration-200 spring-in",
    colors2[alertType],
    className
  ), children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: "h-5 w-5 mt-0.5 flex-shrink-0 spring-hover" }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex-1 min-w-0", children: [
      title && /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-semibold text-sm", children: title }),
      message && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm opacity-90 mt-1", children: message }),
      children
    ] })
  ] });
}
function ModernCard({
  title,
  subtitle,
  icon,
  action,
  variant = "default",
  padding = "md",
  children,
  className,
  ...props
}) {
  const variants = {
    default: "bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 shadow-sm",
    elevated: "bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 shadow-lg",
    outlined: "bg-white dark:bg-gray-900 border-2 border-gray-300 dark:border-gray-700"
  };
  const paddings = {
    none: "",
    sm: "p-4",
    md: "p-6",
    lg: "p-8"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "div",
    {
      className: cn(
        "rounded-xl transition-all duration-200 hover:shadow-md animate-fade-in-scale",
        "card-hover",
        variants[variant],
        className
      ),
      ...props,
      children: [
        (title || subtitle || icon || action) && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-6 pb-0 mb-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3 min-w-0", children: [
            icon && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-2 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 flex-shrink-0", children: icon }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "min-w-0", children: [
              title && /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-semibold text-gray-900 dark:text-gray-100 truncate", children: title }),
              subtitle && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500 dark:text-gray-400 mt-1", children: subtitle })
            ] })
          ] }),
          action && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0", children: action })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn(paddings[padding], title ? "" : paddings[padding]), children })
      ]
    }
  );
}
function ModernBadge({
  children,
  variant = "default",
  size = "md",
  className
}) {
  const variants = {
    default: "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200",
    success: "bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400",
    warning: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400",
    error: "bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400",
    info: "bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400",
    outline: "border border-gray-300 text-gray-700 dark:border-gray-600 dark:text-gray-300",
    destructive: "bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400"
  };
  const sizes = {
    sm: "px-2 py-0.5 text-xs",
    md: "px-2.5 py-1 text-xs",
    lg: "px-3 py-1.5 text-sm"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: cn(
    "inline-flex items-center font-medium rounded-full transition-colors duration-200",
    variants[variant],
    sizes[size],
    className
  ), children });
}
function MetricCard({
  title,
  value,
  subtitle,
  icon,
  trend,
  className
}) {
  const trendColors = {
    up: "text-green-600 dark:text-green-400",
    down: "text-red-600 dark:text-red-400",
    neutral: "text-gray-600 dark:text-gray-400"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { className, children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "min-w-0 flex-1", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm font-medium text-gray-600 dark:text-gray-400 truncate", children: title }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-baseline gap-2 mt-1", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-3xl font-bold text-gray-900 dark:text-gray-100", children: typeof value === "number" ? value.toLocaleString() : value }),
        trend && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: cn(
          "text-sm font-medium",
          trendColors[trend.direction]
        ), children: trend.value })
      ] }),
      subtitle && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mt-1", children: subtitle })
    ] }),
    icon && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 spring-hover", children: icon })
  ] }) });
}
function EmptyState({
  icon,
  title,
  description,
  action,
  className
}) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn(
    "flex flex-col items-center justify-center py-12 px-4 text-center",
    className
  ), children: [
    icon && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-4 rounded-full bg-gray-100 dark:bg-gray-800 text-gray-400 dark:text-gray-500 mb-4", children: icon }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2", children: title }),
    description && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500 dark:text-gray-400 mb-6 max-w-md", children: description }),
    action && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { children: action })
  ] });
}
function PageHeader({ title, subtitle, action, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn(
    "flex items-center justify-between py-6 border-b border-gray-200 dark:border-gray-800",
    className
  ), children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "min-w-0", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold text-gray-900 dark:text-gray-100 truncate", children: title }),
      subtitle && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-lg text-gray-600 dark:text-gray-400 mt-2", children: subtitle })
    ] }),
    action && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0 ml-4", children: action })
  ] });
}
function Section({ title, subtitle, action, className, children }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("section", { className: cn("py-8", className), children: [
    (title || subtitle || action) && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between mb-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "min-w-0", children: [
        title && /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100", children: title }),
        subtitle && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-base text-gray-600 dark:text-gray-400 mt-1", children: subtitle })
      ] }),
      action && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-shrink-0", children: action })
    ] }),
    children
  ] });
}
function Button({
  children,
  variant = "primary",
  size = "md",
  className,
  loading,
  disabled,
  ...props
}) {
  const variants = {
    primary: "bg-blue-600 hover:bg-blue-700 text-white shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm",
    secondary: "bg-gray-100 hover:bg-gray-200 text-gray-900 dark:bg-gray-800 dark:hover:bg-gray-700 dark:text-gray-100 hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm",
    danger: "bg-red-600 hover:bg-red-700 text-white shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm",
    success: "bg-green-600 hover:bg-green-700 text-white shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm",
    outline: "border border-gray-300 dark:border-gray-700 bg-transparent hover:bg-gray-50 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300",
    ghost: "bg-transparent hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300",
    destructive: "bg-red-600 hover:bg-red-700 text-white shadow-sm hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:shadow-sm"
  };
  const sizes = {
    sm: "px-3 py-1.5 text-sm",
    md: "px-4 py-2 text-base",
    lg: "px-6 py-3 text-lg"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "button",
    {
      className: cn(
        "inline-flex items-center justify-center font-medium rounded-lg transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed",
        variants[variant],
        sizes[size],
        className
      ),
      disabled: disabled || loading,
      ...props,
      children: [
        loading && /* @__PURE__ */ jsxRuntimeExports.jsxs("svg", { className: "animate-spin -ml-1 mr-2 h-4 w-4", fill: "none", viewBox: "0 0 24 24", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("circle", { className: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "4" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("path", { className: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" })
        ] }),
        children
      ]
    }
  );
}
const Card$1 = ModernCard;
const Badge$1 = ModernBadge;
function Modal({ open, onClose, onOpenChange, title, children, className }) {
  const handleClose = () => {
    onClose == null ? void 0 : onClose();
    onOpenChange == null ? void 0 : onOpenChange(false);
  };
  if (!open) return null;
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      "div",
      {
        className: "fixed inset-0 bg-black bg-opacity-50 z-40",
        onClick: handleClose
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "fixed inset-0 z-50 flex items-center justify-center p-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(ModernCard, { className: cn("max-w-md w-full max-h-[90vh] overflow-y-auto", className), children: [
      title && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: title }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: handleClose,
            className: "p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400",
            children: /* @__PURE__ */ jsxRuntimeExports.jsx(X, { className: "h-5 w-5" })
          }
        )
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-6", children })
    ] }) })
  ] });
}
function Input({ className, size = "md", ...props }) {
  const sizes = {
    sm: "px-3 py-1.5 text-sm",
    md: "px-4 py-2.5 text-lg",
    lg: "px-6 py-3.5 text-lg"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "input",
    {
      className: cn(
        "w-full rounded-xl border-2 border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900",
        "text-primary placeholder-tertiary",
        "focus:border-brand focus:outline-none focus:ring-4 focus:ring-brand/20",
        "hover:border-gray-300 dark:hover:border-gray-600",
        "transition-all duration-200 ease-out",
        "shadow-sm hover:shadow-md focus:shadow-lg",
        sizes[size],
        className
      ),
      ...props
    }
  );
}
function Label$1({ className, required, children, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "label",
    {
      className: cn(
        "block text-lg font-medium text-gray-900 dark:text-gray-100 mb-2",
        className
      ),
      ...props,
      children: [
        children,
        required && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-red-700 dark:text-red-500 ml-1", children: "*" })
      ]
    }
  );
}
function ContextMenuItem({ children, onClick, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { onClick, className, children });
}
function Progress({ value, max = 100, className }) {
  const percentage = Math.min(value / max * 100, 100);
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2", className), children: /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: "bg-blue-600 h-2 rounded-full transition-all duration-300",
      style: { width: `${percentage}%` }
    }
  ) });
}
function Checkbox({ id, checked, onCheckedChange, disabled, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "input",
    {
      id,
      type: "checkbox",
      checked,
      onChange: (e) => onCheckedChange == null ? void 0 : onCheckedChange(e.target.checked),
      disabled,
      className: cn(
        "h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500",
        disabled && "opacity-50 cursor-not-allowed",
        className
      )
    }
  );
}
function DropdownMenu({ children }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "relative inline-block", children });
}
function DropdownMenuTrigger({ children, asChild, className }) {
  if (asChild) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx(jsxRuntimeExports.Fragment, { children });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className, children });
}
function DropdownMenuContent({ children, align = "end", className }) {
  const alignClass = align === "start" ? "left-0" : align === "end" ? "right-0" : "left-1/2 -translate-x-1/2";
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: cn(
        "absolute z-50 mt-2 min-w-[8rem] overflow-hidden rounded-md border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-1 text-gray-900 dark:text-gray-100 shadow-md",
        alignClass,
        className
      ),
      children
    }
  );
}
function DropdownMenuItem({ children, onClick, disabled, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "button",
    {
      onClick,
      disabled,
      className: cn(
        "relative flex w-full cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-gray-100 dark:hover:bg-gray-800 focus:bg-gray-100 dark:focus:bg-gray-800",
        disabled && "pointer-events-none opacity-50",
        className
      ),
      children
    }
  );
}
const iconSizes = {
  xs: "h-3 w-3",
  sm: "h-4 w-4",
  md: "h-5 w-5",
  lg: "h-6 w-6",
  xl: "h-8 w-8",
  "2xl": "h-10 w-10",
  "3xl": "h-12 w-12"
};
const iconColors = {
  default: "text-secondary",
  primary: "text-primary",
  brand: "text-brand",
  success: "text-success",
  warning: "text-warning",
  danger: "text-danger",
  muted: "text-tertiary"
};
function Icon({
  icon: IconComponent,
  size = "md",
  color = "default",
  className,
  onClick,
  "aria-label": ariaLabel,
  ...props
}) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    IconComponent,
    {
      className: cn(
        iconSizes[size],
        iconColors[color],
        onClick && "cursor-pointer hover:opacity-75 transition-opacity",
        className
      ),
      onClick,
      "aria-label": ariaLabel,
      ...props
    }
  );
}
function StatusIcon$1({
  status,
  size = "md",
  className
}) {
  const statusConfig = {
    success: { icon: CircleCheckBig, color: "success" },
    error: { icon: CircleX, color: "danger" },
    warning: { icon: TriangleAlert, color: "warning" },
    info: { icon: Info, color: "brand" },
    pending: { icon: Clock, color: "muted" }
  };
  const config = statusConfig[status];
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Icon,
    {
      icon: config.icon,
      size,
      color: config.color,
      className
    }
  );
}
function MetricIcon({
  metric,
  size = "lg",
  className
}) {
  const metricConfig = {
    cpu: { icon: Cpu, color: "primary" },
    memory: { icon: HardDrive, color: "primary" },
    activity: { icon: Activity, color: "primary" },
    uptime: { icon: Clock, color: "primary" },
    performance: { icon: Zap, color: "primary" }
  };
  const config = metricConfig[metric];
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Icon,
    {
      icon: config.icon,
      size,
      color: config.color,
      className
    }
  );
}
function ActionIcon({
  action,
  size = "sm",
  onClick,
  className,
  "aria-label": ariaLabel
}) {
  const actionConfig = {
    add: { icon: Plus, color: "success" },
    remove: { icon: Minus, color: "warning" },
    edit: { icon: SquarePen, color: "primary" },
    delete: { icon: Trash2, color: "danger" },
    download: { icon: Download, color: "primary" },
    upload: { icon: Upload, color: "primary" },
    copy: { icon: Copy, color: "primary" },
    view: { icon: Eye, color: "primary" },
    hide: { icon: EyeOff, color: "muted" }
  };
  const config = actionConfig[action];
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Icon,
    {
      icon: config.icon,
      size,
      color: config.color,
      onClick,
      className: cn("interactive-pulse", className),
      "aria-label": ariaLabel || `${action} action`
    }
  );
}
function ChevronIcon({
  direction,
  size = "sm",
  onClick,
  className
}) {
  const chevronConfig = {
    up: ChevronUp,
    down: ChevronDown,
    left: ChevronLeft,
    right: ChevronRight
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Icon,
    {
      icon: chevronConfig[direction],
      size,
      color: "default",
      onClick,
      className: cn(
        "transition-transform duration-200",
        onClick && "hover:scale-110",
        className
      )
    }
  );
}
const Icons = {
  // UI
  Close: X,
  Search
};
async function initErrorReporting() {
}
function reportError(error, context) {
}
class ErrorBoundary extends reactExports.Component {
  constructor(props) {
    super(props);
    __publicField(this, "handleRetry", () => {
      this.setState({ hasError: false, error: void 0, errorInfo: void 0 });
    });
    __publicField(this, "handleGoHome", () => {
      window.location.href = "/";
    });
    this.state = { hasError: false };
  }
  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }
  componentDidCatch(error, errorInfo) {
    logger.error("ErrorBoundary caught an error", error, errorInfo);
    this.setState({
      error,
      errorInfo
    });
    try {
      reportError(error, errorInfo);
    } catch (e) {
      logger.error("Failed to report error", e);
    }
  }
  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        if (typeof this.props.fallback === "function") {
          return this.props.fallback({
            error: this.state.error,
            resetError: this.handleRetry
          });
        }
        return this.props.fallback;
      }
      return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "min-h-screen bg-background flex items-center justify-center p-4", "data-testid": "error-boundary-fallback", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "max-w-lg w-full bg-card border border-gray-200 dark:border-gray-800 rounded-xl shadow-xl p-8 spring-in", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-4 rounded-full bg-red-50 dark:bg-red-900/20 mb-6 inline-flex", children: /* @__PURE__ */ jsxRuntimeExports.jsx(StatusIcon$1, { status: "error", size: "3xl" }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-3xl font-bold text-gray-900 dark:text-gray-100 mb-3", children: "Something went wrong" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-lg text-gray-600 dark:text-gray-400 mb-8", children: "An unexpected error occurred in the application. Please try refreshing the page or contact support if the issue persists." }),
        false,
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3 justify-center", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button,
            {
              onClick: this.handleRetry,
              className: "flex items-center gap-2 spring-hover",
              variant: "primary",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(ActionIcon, { action: "view" }),
                "Try Again"
              ]
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button,
            {
              onClick: this.handleGoHome,
              variant: "secondary",
              className: "flex items-center gap-2 spring-hover",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(ActionIcon, { action: "view" }),
                "Go Home"
              ]
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-6 text-sm text-gray-600 dark:text-gray-400", children: "If this problem persists, please contact support with the error details above." })
      ] }) }) });
    }
    return this.props.children;
  }
}
let toastIdCounter = 0;
const generateId = () => {
  toastIdCounter += 1;
  return `toast-${toastIdCounter}-${Date.now()}`;
};
const useToastStore = create()((set, get) => ({
  toasts: [],
  addToast: (toast2) => {
    const id = generateId();
    const newToast = {
      id,
      duration: 5e3,
      dismissible: true,
      ...toast2
    };
    set((state) => ({
      toasts: [...state.toasts, newToast]
    }));
    if (newToast.duration && newToast.duration > 0) {
      setTimeout(() => {
        get().removeToast(id);
      }, newToast.duration);
    }
    return id;
  },
  removeToast: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((toast2) => toast2.id !== id)
    }));
  },
  clearAllToasts: () => {
    set({ toasts: [] });
  },
  success: (title, message) => {
    return get().addToast({ type: "success", title, message });
  },
  error: (title, message) => {
    return get().addToast({ type: "error", title, message, duration: 8e3 });
  },
  warning: (title, message) => {
    return get().addToast({ type: "warning", title, message });
  },
  info: (title, message) => {
    return get().addToast({ type: "info", title, message });
  }
}));
function Toast({
  type,
  title,
  message,
  duration = 5e3,
  onClose
}) {
  const [isVisible, setIsVisible] = reactExports.useState(true);
  const [isExiting, setIsExiting] = reactExports.useState(false);
  reactExports.useEffect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        handleClose();
      }, duration);
      return () => clearTimeout(timer);
    }
  }, [duration]);
  const handleClose = () => {
    setIsExiting(true);
    setTimeout(() => {
      setIsVisible(false);
      onClose == null ? void 0 : onClose();
    }, 300);
  };
  if (!isVisible) return null;
  const icons = {
    success: CircleCheckBig,
    error: CircleX,
    warning: CircleAlert,
    info: Info
  };
  const colors = {
    success: "bg-green-50 border-green-200 text-green-800 dark:bg-green-950 dark:border-green-800 dark:text-green-200",
    error: "bg-red-50 border-red-200 text-red-800 dark:bg-red-950 dark:border-red-800 dark:text-red-200",
    warning: "bg-yellow-50 border-yellow-200 text-yellow-800 dark:bg-yellow-950 dark:border-yellow-800 dark:text-yellow-200",
    info: "bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-950 dark:border-blue-800 dark:text-blue-200"
  };
  const iconColors2 = {
    success: "text-green-500",
    error: "text-red-500",
    warning: "text-yellow-500",
    info: "text-blue-500"
  };
  const Icon2 = icons[type];
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "div",
    {
      className: cn(
        "flex items-start gap-3 p-4 border rounded-lg shadow-lg transition-all duration-300",
        colors[type],
        isExiting ? "opacity-0 transform translate-x-full" : "opacity-100 transform translate-x-0"
      ),
      role: "alert",
      "aria-live": type === "error" ? "assertive" : "polite",
      children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: cn("h-5 w-5 mt-0.5 flex-shrink-0", iconColors2[type]) }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex-1 min-w-0", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "text-sm font-medium", children: title }),
          message && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm opacity-90 mt-1", children: message })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: handleClose,
            className: "flex-shrink-0 p-1 rounded hover:bg-black/10 dark:hover:bg-white/10 transition-colors",
            "aria-label": "Close notification",
            children: /* @__PURE__ */ jsxRuntimeExports.jsx(X, { className: "h-4 w-4" })
          }
        )
      ]
    }
  );
}
function ToastContainer() {
  const { toasts, removeToast } = useToastStore();
  if (toasts.length === 0) return null;
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: "fixed top-4 right-4 z-50 space-y-2 max-w-sm pointer-events-none",
      "aria-label": "Notifications",
      children: toasts.map((toast2) => /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "pointer-events-auto", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
        Toast,
        {
          type: toast2.type,
          title: toast2.title,
          message: toast2.message,
          duration: 0,
          onClose: () => removeToast(toast2.id)
        }
      ) }, toast2.id))
    }
  );
}
const toast = {
  success: (title, message) => {
    return useToastStore.getState().success(title, message);
  },
  error: (title, message) => {
    return useToastStore.getState().error(title, message);
  },
  warning: (title, message) => {
    return useToastStore.getState().warning(title, message);
  },
  info: (title, message) => {
    return useToastStore.getState().info(title, message);
  }
};
const ToastContext = reactExports.createContext(void 0);
function ToastProvider({ children }) {
  const { addToast, removeToast, clearAllToasts: clearAll } = useToastStore();
  const showToast = reactExports.useCallback((type, title, message, duration) => {
    return addToast({
      type,
      title,
      message,
      duration: duration ?? (type === "error" ? 8e3 : 5e3)
    });
  }, [addToast]);
  const hideToast = reactExports.useCallback((id) => {
    removeToast(id);
  }, [removeToast]);
  const clearAllToasts = reactExports.useCallback(() => {
    clearAll();
  }, [clearAll]);
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(ToastContext.Provider, { value: { showToast, hideToast, clearAllToasts }, children: [
    children,
    /* @__PURE__ */ jsxRuntimeExports.jsx(ToastContainer, {})
  ] });
}
function useToast() {
  const context = reactExports.useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within a ToastProvider");
  }
  return context;
}
function useErrorToast() {
  const { showToast } = useToast();
  return reactExports.useCallback((title, message, duration) => showToast("error", title, message, duration), [showToast]);
}
var util;
(function(util2) {
  util2.assertEqual = (_) => {
  };
  function assertIs(_arg) {
  }
  util2.assertIs = assertIs;
  function assertNever(_x) {
    throw new Error();
  }
  util2.assertNever = assertNever;
  util2.arrayToEnum = (items) => {
    const obj = {};
    for (const item of items) {
      obj[item] = item;
    }
    return obj;
  };
  util2.getValidEnumValues = (obj) => {
    const validKeys = util2.objectKeys(obj).filter((k) => typeof obj[obj[k]] !== "number");
    const filtered = {};
    for (const k of validKeys) {
      filtered[k] = obj[k];
    }
    return util2.objectValues(filtered);
  };
  util2.objectValues = (obj) => {
    return util2.objectKeys(obj).map(function(e) {
      return obj[e];
    });
  };
  util2.objectKeys = typeof Object.keys === "function" ? (obj) => Object.keys(obj) : (object) => {
    const keys = [];
    for (const key in object) {
      if (Object.prototype.hasOwnProperty.call(object, key)) {
        keys.push(key);
      }
    }
    return keys;
  };
  util2.find = (arr, checker) => {
    for (const item of arr) {
      if (checker(item))
        return item;
    }
    return void 0;
  };
  util2.isInteger = typeof Number.isInteger === "function" ? (val) => Number.isInteger(val) : (val) => typeof val === "number" && Number.isFinite(val) && Math.floor(val) === val;
  function joinValues(array, separator = " | ") {
    return array.map((val) => typeof val === "string" ? `'${val}'` : val).join(separator);
  }
  util2.joinValues = joinValues;
  util2.jsonStringifyReplacer = (_, value) => {
    if (typeof value === "bigint") {
      return value.toString();
    }
    return value;
  };
})(util || (util = {}));
var objectUtil;
(function(objectUtil2) {
  objectUtil2.mergeShapes = (first, second) => {
    return {
      ...first,
      ...second
      // second overwrites first
    };
  };
})(objectUtil || (objectUtil = {}));
const ZodParsedType = util.arrayToEnum([
  "string",
  "nan",
  "number",
  "integer",
  "float",
  "boolean",
  "date",
  "bigint",
  "symbol",
  "function",
  "undefined",
  "null",
  "array",
  "object",
  "unknown",
  "promise",
  "void",
  "never",
  "map",
  "set"
]);
const getParsedType = (data) => {
  const t = typeof data;
  switch (t) {
    case "undefined":
      return ZodParsedType.undefined;
    case "string":
      return ZodParsedType.string;
    case "number":
      return Number.isNaN(data) ? ZodParsedType.nan : ZodParsedType.number;
    case "boolean":
      return ZodParsedType.boolean;
    case "function":
      return ZodParsedType.function;
    case "bigint":
      return ZodParsedType.bigint;
    case "symbol":
      return ZodParsedType.symbol;
    case "object":
      if (Array.isArray(data)) {
        return ZodParsedType.array;
      }
      if (data === null) {
        return ZodParsedType.null;
      }
      if (data.then && typeof data.then === "function" && data.catch && typeof data.catch === "function") {
        return ZodParsedType.promise;
      }
      if (typeof Map !== "undefined" && data instanceof Map) {
        return ZodParsedType.map;
      }
      if (typeof Set !== "undefined" && data instanceof Set) {
        return ZodParsedType.set;
      }
      if (typeof Date !== "undefined" && data instanceof Date) {
        return ZodParsedType.date;
      }
      return ZodParsedType.object;
    default:
      return ZodParsedType.unknown;
  }
};
const ZodIssueCode = util.arrayToEnum([
  "invalid_type",
  "invalid_literal",
  "custom",
  "invalid_union",
  "invalid_union_discriminator",
  "invalid_enum_value",
  "unrecognized_keys",
  "invalid_arguments",
  "invalid_return_type",
  "invalid_date",
  "invalid_string",
  "too_small",
  "too_big",
  "invalid_intersection_types",
  "not_multiple_of",
  "not_finite"
]);
class ZodError extends Error {
  get errors() {
    return this.issues;
  }
  constructor(issues) {
    super();
    this.issues = [];
    this.addIssue = (sub) => {
      this.issues = [...this.issues, sub];
    };
    this.addIssues = (subs = []) => {
      this.issues = [...this.issues, ...subs];
    };
    const actualProto = new.target.prototype;
    if (Object.setPrototypeOf) {
      Object.setPrototypeOf(this, actualProto);
    } else {
      this.__proto__ = actualProto;
    }
    this.name = "ZodError";
    this.issues = issues;
  }
  format(_mapper) {
    const mapper = _mapper || function(issue) {
      return issue.message;
    };
    const fieldErrors = { _errors: [] };
    const processError = (error) => {
      for (const issue of error.issues) {
        if (issue.code === "invalid_union") {
          issue.unionErrors.map(processError);
        } else if (issue.code === "invalid_return_type") {
          processError(issue.returnTypeError);
        } else if (issue.code === "invalid_arguments") {
          processError(issue.argumentsError);
        } else if (issue.path.length === 0) {
          fieldErrors._errors.push(mapper(issue));
        } else {
          let curr = fieldErrors;
          let i = 0;
          while (i < issue.path.length) {
            const el = issue.path[i];
            const terminal = i === issue.path.length - 1;
            if (!terminal) {
              curr[el] = curr[el] || { _errors: [] };
            } else {
              curr[el] = curr[el] || { _errors: [] };
              curr[el]._errors.push(mapper(issue));
            }
            curr = curr[el];
            i++;
          }
        }
      }
    };
    processError(this);
    return fieldErrors;
  }
  static assert(value) {
    if (!(value instanceof ZodError)) {
      throw new Error(`Not a ZodError: ${value}`);
    }
  }
  toString() {
    return this.message;
  }
  get message() {
    return JSON.stringify(this.issues, util.jsonStringifyReplacer, 2);
  }
  get isEmpty() {
    return this.issues.length === 0;
  }
  flatten(mapper = (issue) => issue.message) {
    const fieldErrors = {};
    const formErrors = [];
    for (const sub of this.issues) {
      if (sub.path.length > 0) {
        const firstEl = sub.path[0];
        fieldErrors[firstEl] = fieldErrors[firstEl] || [];
        fieldErrors[firstEl].push(mapper(sub));
      } else {
        formErrors.push(mapper(sub));
      }
    }
    return { formErrors, fieldErrors };
  }
  get formErrors() {
    return this.flatten();
  }
}
ZodError.create = (issues) => {
  const error = new ZodError(issues);
  return error;
};
const errorMap = (issue, _ctx) => {
  let message;
  switch (issue.code) {
    case ZodIssueCode.invalid_type:
      if (issue.received === ZodParsedType.undefined) {
        message = "Required";
      } else {
        message = `Expected ${issue.expected}, received ${issue.received}`;
      }
      break;
    case ZodIssueCode.invalid_literal:
      message = `Invalid literal value, expected ${JSON.stringify(issue.expected, util.jsonStringifyReplacer)}`;
      break;
    case ZodIssueCode.unrecognized_keys:
      message = `Unrecognized key(s) in object: ${util.joinValues(issue.keys, ", ")}`;
      break;
    case ZodIssueCode.invalid_union:
      message = `Invalid input`;
      break;
    case ZodIssueCode.invalid_union_discriminator:
      message = `Invalid discriminator value. Expected ${util.joinValues(issue.options)}`;
      break;
    case ZodIssueCode.invalid_enum_value:
      message = `Invalid enum value. Expected ${util.joinValues(issue.options)}, received '${issue.received}'`;
      break;
    case ZodIssueCode.invalid_arguments:
      message = `Invalid function arguments`;
      break;
    case ZodIssueCode.invalid_return_type:
      message = `Invalid function return type`;
      break;
    case ZodIssueCode.invalid_date:
      message = `Invalid date`;
      break;
    case ZodIssueCode.invalid_string:
      if (typeof issue.validation === "object") {
        if ("includes" in issue.validation) {
          message = `Invalid input: must include "${issue.validation.includes}"`;
          if (typeof issue.validation.position === "number") {
            message = `${message} at one or more positions greater than or equal to ${issue.validation.position}`;
          }
        } else if ("startsWith" in issue.validation) {
          message = `Invalid input: must start with "${issue.validation.startsWith}"`;
        } else if ("endsWith" in issue.validation) {
          message = `Invalid input: must end with "${issue.validation.endsWith}"`;
        } else {
          util.assertNever(issue.validation);
        }
      } else if (issue.validation !== "regex") {
        message = `Invalid ${issue.validation}`;
      } else {
        message = "Invalid";
      }
      break;
    case ZodIssueCode.too_small:
      if (issue.type === "array")
        message = `Array must contain ${issue.exact ? "exactly" : issue.inclusive ? `at least` : `more than`} ${issue.minimum} element(s)`;
      else if (issue.type === "string")
        message = `String must contain ${issue.exact ? "exactly" : issue.inclusive ? `at least` : `over`} ${issue.minimum} character(s)`;
      else if (issue.type === "number")
        message = `Number must be ${issue.exact ? `exactly equal to ` : issue.inclusive ? `greater than or equal to ` : `greater than `}${issue.minimum}`;
      else if (issue.type === "bigint")
        message = `Number must be ${issue.exact ? `exactly equal to ` : issue.inclusive ? `greater than or equal to ` : `greater than `}${issue.minimum}`;
      else if (issue.type === "date")
        message = `Date must be ${issue.exact ? `exactly equal to ` : issue.inclusive ? `greater than or equal to ` : `greater than `}${new Date(Number(issue.minimum))}`;
      else
        message = "Invalid input";
      break;
    case ZodIssueCode.too_big:
      if (issue.type === "array")
        message = `Array must contain ${issue.exact ? `exactly` : issue.inclusive ? `at most` : `less than`} ${issue.maximum} element(s)`;
      else if (issue.type === "string")
        message = `String must contain ${issue.exact ? `exactly` : issue.inclusive ? `at most` : `under`} ${issue.maximum} character(s)`;
      else if (issue.type === "number")
        message = `Number must be ${issue.exact ? `exactly` : issue.inclusive ? `less than or equal to` : `less than`} ${issue.maximum}`;
      else if (issue.type === "bigint")
        message = `BigInt must be ${issue.exact ? `exactly` : issue.inclusive ? `less than or equal to` : `less than`} ${issue.maximum}`;
      else if (issue.type === "date")
        message = `Date must be ${issue.exact ? `exactly` : issue.inclusive ? `smaller than or equal to` : `smaller than`} ${new Date(Number(issue.maximum))}`;
      else
        message = "Invalid input";
      break;
    case ZodIssueCode.custom:
      message = `Invalid input`;
      break;
    case ZodIssueCode.invalid_intersection_types:
      message = `Intersection results could not be merged`;
      break;
    case ZodIssueCode.not_multiple_of:
      message = `Number must be a multiple of ${issue.multipleOf}`;
      break;
    case ZodIssueCode.not_finite:
      message = "Number must be finite";
      break;
    default:
      message = _ctx.defaultError;
      util.assertNever(issue);
  }
  return { message };
};
let overrideErrorMap = errorMap;
function getErrorMap() {
  return overrideErrorMap;
}
const makeIssue = (params) => {
  const { data, path, errorMaps, issueData } = params;
  const fullPath = [...path, ...issueData.path || []];
  const fullIssue = {
    ...issueData,
    path: fullPath
  };
  if (issueData.message !== void 0) {
    return {
      ...issueData,
      path: fullPath,
      message: issueData.message
    };
  }
  let errorMessage = "";
  const maps = errorMaps.filter((m) => !!m).slice().reverse();
  for (const map of maps) {
    errorMessage = map(fullIssue, { data, defaultError: errorMessage }).message;
  }
  return {
    ...issueData,
    path: fullPath,
    message: errorMessage
  };
};
function addIssueToContext(ctx, issueData) {
  const overrideMap = getErrorMap();
  const issue = makeIssue({
    issueData,
    data: ctx.data,
    path: ctx.path,
    errorMaps: [
      ctx.common.contextualErrorMap,
      // contextual error map is first priority
      ctx.schemaErrorMap,
      // then schema-bound map if available
      overrideMap,
      // then global override map
      overrideMap === errorMap ? void 0 : errorMap
      // then global default map
    ].filter((x) => !!x)
  });
  ctx.common.issues.push(issue);
}
class ParseStatus {
  constructor() {
    this.value = "valid";
  }
  dirty() {
    if (this.value === "valid")
      this.value = "dirty";
  }
  abort() {
    if (this.value !== "aborted")
      this.value = "aborted";
  }
  static mergeArray(status, results) {
    const arrayValue = [];
    for (const s of results) {
      if (s.status === "aborted")
        return INVALID;
      if (s.status === "dirty")
        status.dirty();
      arrayValue.push(s.value);
    }
    return { status: status.value, value: arrayValue };
  }
  static async mergeObjectAsync(status, pairs) {
    const syncPairs = [];
    for (const pair of pairs) {
      const key = await pair.key;
      const value = await pair.value;
      syncPairs.push({
        key,
        value
      });
    }
    return ParseStatus.mergeObjectSync(status, syncPairs);
  }
  static mergeObjectSync(status, pairs) {
    const finalObject = {};
    for (const pair of pairs) {
      const { key, value } = pair;
      if (key.status === "aborted")
        return INVALID;
      if (value.status === "aborted")
        return INVALID;
      if (key.status === "dirty")
        status.dirty();
      if (value.status === "dirty")
        status.dirty();
      if (key.value !== "__proto__" && (typeof value.value !== "undefined" || pair.alwaysSet)) {
        finalObject[key.value] = value.value;
      }
    }
    return { status: status.value, value: finalObject };
  }
}
const INVALID = Object.freeze({
  status: "aborted"
});
const DIRTY = (value) => ({ status: "dirty", value });
const OK = (value) => ({ status: "valid", value });
const isAborted = (x) => x.status === "aborted";
const isDirty = (x) => x.status === "dirty";
const isValid = (x) => x.status === "valid";
const isAsync = (x) => typeof Promise !== "undefined" && x instanceof Promise;
var errorUtil;
(function(errorUtil2) {
  errorUtil2.errToObj = (message) => typeof message === "string" ? { message } : message || {};
  errorUtil2.toString = (message) => typeof message === "string" ? message : message == null ? void 0 : message.message;
})(errorUtil || (errorUtil = {}));
class ParseInputLazyPath {
  constructor(parent, value, path, key) {
    this._cachedPath = [];
    this.parent = parent;
    this.data = value;
    this._path = path;
    this._key = key;
  }
  get path() {
    if (!this._cachedPath.length) {
      if (Array.isArray(this._key)) {
        this._cachedPath.push(...this._path, ...this._key);
      } else {
        this._cachedPath.push(...this._path, this._key);
      }
    }
    return this._cachedPath;
  }
}
const handleResult = (ctx, result) => {
  if (isValid(result)) {
    return { success: true, data: result.value };
  } else {
    if (!ctx.common.issues.length) {
      throw new Error("Validation failed but no issues detected.");
    }
    return {
      success: false,
      get error() {
        if (this._error)
          return this._error;
        const error = new ZodError(ctx.common.issues);
        this._error = error;
        return this._error;
      }
    };
  }
};
function processCreateParams(params) {
  if (!params)
    return {};
  const { errorMap: errorMap2, invalid_type_error, required_error, description } = params;
  if (errorMap2 && (invalid_type_error || required_error)) {
    throw new Error(`Can't use "invalid_type_error" or "required_error" in conjunction with custom error map.`);
  }
  if (errorMap2)
    return { errorMap: errorMap2, description };
  const customMap = (iss, ctx) => {
    const { message } = params;
    if (iss.code === "invalid_enum_value") {
      return { message: message ?? ctx.defaultError };
    }
    if (typeof ctx.data === "undefined") {
      return { message: message ?? required_error ?? ctx.defaultError };
    }
    if (iss.code !== "invalid_type")
      return { message: ctx.defaultError };
    return { message: message ?? invalid_type_error ?? ctx.defaultError };
  };
  return { errorMap: customMap, description };
}
class ZodType {
  get description() {
    return this._def.description;
  }
  _getType(input) {
    return getParsedType(input.data);
  }
  _getOrReturnCtx(input, ctx) {
    return ctx || {
      common: input.parent.common,
      data: input.data,
      parsedType: getParsedType(input.data),
      schemaErrorMap: this._def.errorMap,
      path: input.path,
      parent: input.parent
    };
  }
  _processInputParams(input) {
    return {
      status: new ParseStatus(),
      ctx: {
        common: input.parent.common,
        data: input.data,
        parsedType: getParsedType(input.data),
        schemaErrorMap: this._def.errorMap,
        path: input.path,
        parent: input.parent
      }
    };
  }
  _parseSync(input) {
    const result = this._parse(input);
    if (isAsync(result)) {
      throw new Error("Synchronous parse encountered promise.");
    }
    return result;
  }
  _parseAsync(input) {
    const result = this._parse(input);
    return Promise.resolve(result);
  }
  parse(data, params) {
    const result = this.safeParse(data, params);
    if (result.success)
      return result.data;
    throw result.error;
  }
  safeParse(data, params) {
    const ctx = {
      common: {
        issues: [],
        async: (params == null ? void 0 : params.async) ?? false,
        contextualErrorMap: params == null ? void 0 : params.errorMap
      },
      path: (params == null ? void 0 : params.path) || [],
      schemaErrorMap: this._def.errorMap,
      parent: null,
      data,
      parsedType: getParsedType(data)
    };
    const result = this._parseSync({ data, path: ctx.path, parent: ctx });
    return handleResult(ctx, result);
  }
  "~validate"(data) {
    var _a, _b;
    const ctx = {
      common: {
        issues: [],
        async: !!this["~standard"].async
      },
      path: [],
      schemaErrorMap: this._def.errorMap,
      parent: null,
      data,
      parsedType: getParsedType(data)
    };
    if (!this["~standard"].async) {
      try {
        const result = this._parseSync({ data, path: [], parent: ctx });
        return isValid(result) ? {
          value: result.value
        } : {
          issues: ctx.common.issues
        };
      } catch (err) {
        if ((_b = (_a = err == null ? void 0 : err.message) == null ? void 0 : _a.toLowerCase()) == null ? void 0 : _b.includes("encountered")) {
          this["~standard"].async = true;
        }
        ctx.common = {
          issues: [],
          async: true
        };
      }
    }
    return this._parseAsync({ data, path: [], parent: ctx }).then((result) => isValid(result) ? {
      value: result.value
    } : {
      issues: ctx.common.issues
    });
  }
  async parseAsync(data, params) {
    const result = await this.safeParseAsync(data, params);
    if (result.success)
      return result.data;
    throw result.error;
  }
  async safeParseAsync(data, params) {
    const ctx = {
      common: {
        issues: [],
        contextualErrorMap: params == null ? void 0 : params.errorMap,
        async: true
      },
      path: (params == null ? void 0 : params.path) || [],
      schemaErrorMap: this._def.errorMap,
      parent: null,
      data,
      parsedType: getParsedType(data)
    };
    const maybeAsyncResult = this._parse({ data, path: ctx.path, parent: ctx });
    const result = await (isAsync(maybeAsyncResult) ? maybeAsyncResult : Promise.resolve(maybeAsyncResult));
    return handleResult(ctx, result);
  }
  refine(check, message) {
    const getIssueProperties = (val) => {
      if (typeof message === "string" || typeof message === "undefined") {
        return { message };
      } else if (typeof message === "function") {
        return message(val);
      } else {
        return message;
      }
    };
    return this._refinement((val, ctx) => {
      const result = check(val);
      const setError = () => ctx.addIssue({
        code: ZodIssueCode.custom,
        ...getIssueProperties(val)
      });
      if (typeof Promise !== "undefined" && result instanceof Promise) {
        return result.then((data) => {
          if (!data) {
            setError();
            return false;
          } else {
            return true;
          }
        });
      }
      if (!result) {
        setError();
        return false;
      } else {
        return true;
      }
    });
  }
  refinement(check, refinementData) {
    return this._refinement((val, ctx) => {
      if (!check(val)) {
        ctx.addIssue(typeof refinementData === "function" ? refinementData(val, ctx) : refinementData);
        return false;
      } else {
        return true;
      }
    });
  }
  _refinement(refinement) {
    return new ZodEffects({
      schema: this,
      typeName: ZodFirstPartyTypeKind.ZodEffects,
      effect: { type: "refinement", refinement }
    });
  }
  superRefine(refinement) {
    return this._refinement(refinement);
  }
  constructor(def) {
    this.spa = this.safeParseAsync;
    this._def = def;
    this.parse = this.parse.bind(this);
    this.safeParse = this.safeParse.bind(this);
    this.parseAsync = this.parseAsync.bind(this);
    this.safeParseAsync = this.safeParseAsync.bind(this);
    this.spa = this.spa.bind(this);
    this.refine = this.refine.bind(this);
    this.refinement = this.refinement.bind(this);
    this.superRefine = this.superRefine.bind(this);
    this.optional = this.optional.bind(this);
    this.nullable = this.nullable.bind(this);
    this.nullish = this.nullish.bind(this);
    this.array = this.array.bind(this);
    this.promise = this.promise.bind(this);
    this.or = this.or.bind(this);
    this.and = this.and.bind(this);
    this.transform = this.transform.bind(this);
    this.brand = this.brand.bind(this);
    this.default = this.default.bind(this);
    this.catch = this.catch.bind(this);
    this.describe = this.describe.bind(this);
    this.pipe = this.pipe.bind(this);
    this.readonly = this.readonly.bind(this);
    this.isNullable = this.isNullable.bind(this);
    this.isOptional = this.isOptional.bind(this);
    this["~standard"] = {
      version: 1,
      vendor: "zod",
      validate: (data) => this["~validate"](data)
    };
  }
  optional() {
    return ZodOptional.create(this, this._def);
  }
  nullable() {
    return ZodNullable.create(this, this._def);
  }
  nullish() {
    return this.nullable().optional();
  }
  array() {
    return ZodArray.create(this);
  }
  promise() {
    return ZodPromise.create(this, this._def);
  }
  or(option) {
    return ZodUnion.create([this, option], this._def);
  }
  and(incoming) {
    return ZodIntersection.create(this, incoming, this._def);
  }
  transform(transform) {
    return new ZodEffects({
      ...processCreateParams(this._def),
      schema: this,
      typeName: ZodFirstPartyTypeKind.ZodEffects,
      effect: { type: "transform", transform }
    });
  }
  default(def) {
    const defaultValueFunc = typeof def === "function" ? def : () => def;
    return new ZodDefault({
      ...processCreateParams(this._def),
      innerType: this,
      defaultValue: defaultValueFunc,
      typeName: ZodFirstPartyTypeKind.ZodDefault
    });
  }
  brand() {
    return new ZodBranded({
      typeName: ZodFirstPartyTypeKind.ZodBranded,
      type: this,
      ...processCreateParams(this._def)
    });
  }
  catch(def) {
    const catchValueFunc = typeof def === "function" ? def : () => def;
    return new ZodCatch({
      ...processCreateParams(this._def),
      innerType: this,
      catchValue: catchValueFunc,
      typeName: ZodFirstPartyTypeKind.ZodCatch
    });
  }
  describe(description) {
    const This = this.constructor;
    return new This({
      ...this._def,
      description
    });
  }
  pipe(target) {
    return ZodPipeline.create(this, target);
  }
  readonly() {
    return ZodReadonly.create(this);
  }
  isOptional() {
    return this.safeParse(void 0).success;
  }
  isNullable() {
    return this.safeParse(null).success;
  }
}
const cuidRegex = /^c[^\s-]{8,}$/i;
const cuid2Regex = /^[0-9a-z]+$/;
const ulidRegex = /^[0-9A-HJKMNP-TV-Z]{26}$/i;
const uuidRegex = /^[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}$/i;
const nanoidRegex = /^[a-z0-9_-]{21}$/i;
const jwtRegex = /^[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]*$/;
const durationRegex = /^[-+]?P(?!$)(?:(?:[-+]?\d+Y)|(?:[-+]?\d+[.,]\d+Y$))?(?:(?:[-+]?\d+M)|(?:[-+]?\d+[.,]\d+M$))?(?:(?:[-+]?\d+W)|(?:[-+]?\d+[.,]\d+W$))?(?:(?:[-+]?\d+D)|(?:[-+]?\d+[.,]\d+D$))?(?:T(?=[\d+-])(?:(?:[-+]?\d+H)|(?:[-+]?\d+[.,]\d+H$))?(?:(?:[-+]?\d+M)|(?:[-+]?\d+[.,]\d+M$))?(?:[-+]?\d+(?:[.,]\d+)?S)?)??$/;
const emailRegex = /^(?!\.)(?!.*\.\.)([A-Z0-9_'+\-\.]*)[A-Z0-9_+-]@([A-Z0-9][A-Z0-9\-]*\.)+[A-Z]{2,}$/i;
const _emojiRegex = `^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$`;
let emojiRegex;
const ipv4Regex = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/;
const ipv4CidrRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/(3[0-2]|[12]?[0-9])$/;
const ipv6Regex = /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$/;
const ipv6CidrRegex = /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/;
const base64Regex = /^([0-9a-zA-Z+/]{4})*(([0-9a-zA-Z+/]{2}==)|([0-9a-zA-Z+/]{3}=))?$/;
const base64urlRegex = /^([0-9a-zA-Z-_]{4})*(([0-9a-zA-Z-_]{2}(==)?)|([0-9a-zA-Z-_]{3}(=)?))?$/;
const dateRegexSource = `((\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-((0[13578]|1[02])-(0[1-9]|[12]\\d|3[01])|(0[469]|11)-(0[1-9]|[12]\\d|30)|(02)-(0[1-9]|1\\d|2[0-8])))`;
const dateRegex = new RegExp(`^${dateRegexSource}$`);
function timeRegexSource(args) {
  let secondsRegexSource = `[0-5]\\d`;
  if (args.precision) {
    secondsRegexSource = `${secondsRegexSource}\\.\\d{${args.precision}}`;
  } else if (args.precision == null) {
    secondsRegexSource = `${secondsRegexSource}(\\.\\d+)?`;
  }
  const secondsQuantifier = args.precision ? "+" : "?";
  return `([01]\\d|2[0-3]):[0-5]\\d(:${secondsRegexSource})${secondsQuantifier}`;
}
function timeRegex(args) {
  return new RegExp(`^${timeRegexSource(args)}$`);
}
function datetimeRegex(args) {
  let regex = `${dateRegexSource}T${timeRegexSource(args)}`;
  const opts = [];
  opts.push(args.local ? `Z?` : `Z`);
  if (args.offset)
    opts.push(`([+-]\\d{2}:?\\d{2})`);
  regex = `${regex}(${opts.join("|")})`;
  return new RegExp(`^${regex}$`);
}
function isValidIP(ip, version) {
  if ((version === "v4" || !version) && ipv4Regex.test(ip)) {
    return true;
  }
  if ((version === "v6" || !version) && ipv6Regex.test(ip)) {
    return true;
  }
  return false;
}
function isValidJWT(jwt, alg) {
  if (!jwtRegex.test(jwt))
    return false;
  try {
    const [header] = jwt.split(".");
    if (!header)
      return false;
    const base64 = header.replace(/-/g, "+").replace(/_/g, "/").padEnd(header.length + (4 - header.length % 4) % 4, "=");
    const decoded = JSON.parse(atob(base64));
    if (typeof decoded !== "object" || decoded === null)
      return false;
    if ("typ" in decoded && (decoded == null ? void 0 : decoded.typ) !== "JWT")
      return false;
    if (!decoded.alg)
      return false;
    if (alg && decoded.alg !== alg)
      return false;
    return true;
  } catch {
    return false;
  }
}
function isValidCidr(ip, version) {
  if ((version === "v4" || !version) && ipv4CidrRegex.test(ip)) {
    return true;
  }
  if ((version === "v6" || !version) && ipv6CidrRegex.test(ip)) {
    return true;
  }
  return false;
}
class ZodString extends ZodType {
  _parse(input) {
    if (this._def.coerce) {
      input.data = String(input.data);
    }
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.string) {
      const ctx2 = this._getOrReturnCtx(input);
      addIssueToContext(ctx2, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.string,
        received: ctx2.parsedType
      });
      return INVALID;
    }
    const status = new ParseStatus();
    let ctx = void 0;
    for (const check of this._def.checks) {
      if (check.kind === "min") {
        if (input.data.length < check.value) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_small,
            minimum: check.value,
            type: "string",
            inclusive: true,
            exact: false,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "max") {
        if (input.data.length > check.value) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_big,
            maximum: check.value,
            type: "string",
            inclusive: true,
            exact: false,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "length") {
        const tooBig = input.data.length > check.value;
        const tooSmall = input.data.length < check.value;
        if (tooBig || tooSmall) {
          ctx = this._getOrReturnCtx(input, ctx);
          if (tooBig) {
            addIssueToContext(ctx, {
              code: ZodIssueCode.too_big,
              maximum: check.value,
              type: "string",
              inclusive: true,
              exact: true,
              message: check.message
            });
          } else if (tooSmall) {
            addIssueToContext(ctx, {
              code: ZodIssueCode.too_small,
              minimum: check.value,
              type: "string",
              inclusive: true,
              exact: true,
              message: check.message
            });
          }
          status.dirty();
        }
      } else if (check.kind === "email") {
        if (!emailRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "email",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "emoji") {
        if (!emojiRegex) {
          emojiRegex = new RegExp(_emojiRegex, "u");
        }
        if (!emojiRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "emoji",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "uuid") {
        if (!uuidRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "uuid",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "nanoid") {
        if (!nanoidRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "nanoid",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "cuid") {
        if (!cuidRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "cuid",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "cuid2") {
        if (!cuid2Regex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "cuid2",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "ulid") {
        if (!ulidRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "ulid",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "url") {
        try {
          new URL(input.data);
        } catch {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "url",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "regex") {
        check.regex.lastIndex = 0;
        const testResult = check.regex.test(input.data);
        if (!testResult) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "regex",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "trim") {
        input.data = input.data.trim();
      } else if (check.kind === "includes") {
        if (!input.data.includes(check.value, check.position)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_string,
            validation: { includes: check.value, position: check.position },
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "toLowerCase") {
        input.data = input.data.toLowerCase();
      } else if (check.kind === "toUpperCase") {
        input.data = input.data.toUpperCase();
      } else if (check.kind === "startsWith") {
        if (!input.data.startsWith(check.value)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_string,
            validation: { startsWith: check.value },
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "endsWith") {
        if (!input.data.endsWith(check.value)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_string,
            validation: { endsWith: check.value },
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "datetime") {
        const regex = datetimeRegex(check);
        if (!regex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_string,
            validation: "datetime",
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "date") {
        const regex = dateRegex;
        if (!regex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_string,
            validation: "date",
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "time") {
        const regex = timeRegex(check);
        if (!regex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_string,
            validation: "time",
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "duration") {
        if (!durationRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "duration",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "ip") {
        if (!isValidIP(input.data, check.version)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "ip",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "jwt") {
        if (!isValidJWT(input.data, check.alg)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "jwt",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "cidr") {
        if (!isValidCidr(input.data, check.version)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "cidr",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "base64") {
        if (!base64Regex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "base64",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "base64url") {
        if (!base64urlRegex.test(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            validation: "base64url",
            code: ZodIssueCode.invalid_string,
            message: check.message
          });
          status.dirty();
        }
      } else {
        util.assertNever(check);
      }
    }
    return { status: status.value, value: input.data };
  }
  _regex(regex, validation, message) {
    return this.refinement((data) => regex.test(data), {
      validation,
      code: ZodIssueCode.invalid_string,
      ...errorUtil.errToObj(message)
    });
  }
  _addCheck(check) {
    return new ZodString({
      ...this._def,
      checks: [...this._def.checks, check]
    });
  }
  email(message) {
    return this._addCheck({ kind: "email", ...errorUtil.errToObj(message) });
  }
  url(message) {
    return this._addCheck({ kind: "url", ...errorUtil.errToObj(message) });
  }
  emoji(message) {
    return this._addCheck({ kind: "emoji", ...errorUtil.errToObj(message) });
  }
  uuid(message) {
    return this._addCheck({ kind: "uuid", ...errorUtil.errToObj(message) });
  }
  nanoid(message) {
    return this._addCheck({ kind: "nanoid", ...errorUtil.errToObj(message) });
  }
  cuid(message) {
    return this._addCheck({ kind: "cuid", ...errorUtil.errToObj(message) });
  }
  cuid2(message) {
    return this._addCheck({ kind: "cuid2", ...errorUtil.errToObj(message) });
  }
  ulid(message) {
    return this._addCheck({ kind: "ulid", ...errorUtil.errToObj(message) });
  }
  base64(message) {
    return this._addCheck({ kind: "base64", ...errorUtil.errToObj(message) });
  }
  base64url(message) {
    return this._addCheck({
      kind: "base64url",
      ...errorUtil.errToObj(message)
    });
  }
  jwt(options) {
    return this._addCheck({ kind: "jwt", ...errorUtil.errToObj(options) });
  }
  ip(options) {
    return this._addCheck({ kind: "ip", ...errorUtil.errToObj(options) });
  }
  cidr(options) {
    return this._addCheck({ kind: "cidr", ...errorUtil.errToObj(options) });
  }
  datetime(options) {
    if (typeof options === "string") {
      return this._addCheck({
        kind: "datetime",
        precision: null,
        offset: false,
        local: false,
        message: options
      });
    }
    return this._addCheck({
      kind: "datetime",
      precision: typeof (options == null ? void 0 : options.precision) === "undefined" ? null : options == null ? void 0 : options.precision,
      offset: (options == null ? void 0 : options.offset) ?? false,
      local: (options == null ? void 0 : options.local) ?? false,
      ...errorUtil.errToObj(options == null ? void 0 : options.message)
    });
  }
  date(message) {
    return this._addCheck({ kind: "date", message });
  }
  time(options) {
    if (typeof options === "string") {
      return this._addCheck({
        kind: "time",
        precision: null,
        message: options
      });
    }
    return this._addCheck({
      kind: "time",
      precision: typeof (options == null ? void 0 : options.precision) === "undefined" ? null : options == null ? void 0 : options.precision,
      ...errorUtil.errToObj(options == null ? void 0 : options.message)
    });
  }
  duration(message) {
    return this._addCheck({ kind: "duration", ...errorUtil.errToObj(message) });
  }
  regex(regex, message) {
    return this._addCheck({
      kind: "regex",
      regex,
      ...errorUtil.errToObj(message)
    });
  }
  includes(value, options) {
    return this._addCheck({
      kind: "includes",
      value,
      position: options == null ? void 0 : options.position,
      ...errorUtil.errToObj(options == null ? void 0 : options.message)
    });
  }
  startsWith(value, message) {
    return this._addCheck({
      kind: "startsWith",
      value,
      ...errorUtil.errToObj(message)
    });
  }
  endsWith(value, message) {
    return this._addCheck({
      kind: "endsWith",
      value,
      ...errorUtil.errToObj(message)
    });
  }
  min(minLength, message) {
    return this._addCheck({
      kind: "min",
      value: minLength,
      ...errorUtil.errToObj(message)
    });
  }
  max(maxLength, message) {
    return this._addCheck({
      kind: "max",
      value: maxLength,
      ...errorUtil.errToObj(message)
    });
  }
  length(len, message) {
    return this._addCheck({
      kind: "length",
      value: len,
      ...errorUtil.errToObj(message)
    });
  }
  /**
   * Equivalent to `.min(1)`
   */
  nonempty(message) {
    return this.min(1, errorUtil.errToObj(message));
  }
  trim() {
    return new ZodString({
      ...this._def,
      checks: [...this._def.checks, { kind: "trim" }]
    });
  }
  toLowerCase() {
    return new ZodString({
      ...this._def,
      checks: [...this._def.checks, { kind: "toLowerCase" }]
    });
  }
  toUpperCase() {
    return new ZodString({
      ...this._def,
      checks: [...this._def.checks, { kind: "toUpperCase" }]
    });
  }
  get isDatetime() {
    return !!this._def.checks.find((ch) => ch.kind === "datetime");
  }
  get isDate() {
    return !!this._def.checks.find((ch) => ch.kind === "date");
  }
  get isTime() {
    return !!this._def.checks.find((ch) => ch.kind === "time");
  }
  get isDuration() {
    return !!this._def.checks.find((ch) => ch.kind === "duration");
  }
  get isEmail() {
    return !!this._def.checks.find((ch) => ch.kind === "email");
  }
  get isURL() {
    return !!this._def.checks.find((ch) => ch.kind === "url");
  }
  get isEmoji() {
    return !!this._def.checks.find((ch) => ch.kind === "emoji");
  }
  get isUUID() {
    return !!this._def.checks.find((ch) => ch.kind === "uuid");
  }
  get isNANOID() {
    return !!this._def.checks.find((ch) => ch.kind === "nanoid");
  }
  get isCUID() {
    return !!this._def.checks.find((ch) => ch.kind === "cuid");
  }
  get isCUID2() {
    return !!this._def.checks.find((ch) => ch.kind === "cuid2");
  }
  get isULID() {
    return !!this._def.checks.find((ch) => ch.kind === "ulid");
  }
  get isIP() {
    return !!this._def.checks.find((ch) => ch.kind === "ip");
  }
  get isCIDR() {
    return !!this._def.checks.find((ch) => ch.kind === "cidr");
  }
  get isBase64() {
    return !!this._def.checks.find((ch) => ch.kind === "base64");
  }
  get isBase64url() {
    return !!this._def.checks.find((ch) => ch.kind === "base64url");
  }
  get minLength() {
    let min = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "min") {
        if (min === null || ch.value > min)
          min = ch.value;
      }
    }
    return min;
  }
  get maxLength() {
    let max = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "max") {
        if (max === null || ch.value < max)
          max = ch.value;
      }
    }
    return max;
  }
}
ZodString.create = (params) => {
  return new ZodString({
    checks: [],
    typeName: ZodFirstPartyTypeKind.ZodString,
    coerce: (params == null ? void 0 : params.coerce) ?? false,
    ...processCreateParams(params)
  });
};
function floatSafeRemainder(val, step) {
  const valDecCount = (val.toString().split(".")[1] || "").length;
  const stepDecCount = (step.toString().split(".")[1] || "").length;
  const decCount = valDecCount > stepDecCount ? valDecCount : stepDecCount;
  const valInt = Number.parseInt(val.toFixed(decCount).replace(".", ""));
  const stepInt = Number.parseInt(step.toFixed(decCount).replace(".", ""));
  return valInt % stepInt / 10 ** decCount;
}
class ZodNumber extends ZodType {
  constructor() {
    super(...arguments);
    this.min = this.gte;
    this.max = this.lte;
    this.step = this.multipleOf;
  }
  _parse(input) {
    if (this._def.coerce) {
      input.data = Number(input.data);
    }
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.number) {
      const ctx2 = this._getOrReturnCtx(input);
      addIssueToContext(ctx2, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.number,
        received: ctx2.parsedType
      });
      return INVALID;
    }
    let ctx = void 0;
    const status = new ParseStatus();
    for (const check of this._def.checks) {
      if (check.kind === "int") {
        if (!util.isInteger(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.invalid_type,
            expected: "integer",
            received: "float",
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "min") {
        const tooSmall = check.inclusive ? input.data < check.value : input.data <= check.value;
        if (tooSmall) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_small,
            minimum: check.value,
            type: "number",
            inclusive: check.inclusive,
            exact: false,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "max") {
        const tooBig = check.inclusive ? input.data > check.value : input.data >= check.value;
        if (tooBig) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_big,
            maximum: check.value,
            type: "number",
            inclusive: check.inclusive,
            exact: false,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "multipleOf") {
        if (floatSafeRemainder(input.data, check.value) !== 0) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.not_multiple_of,
            multipleOf: check.value,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "finite") {
        if (!Number.isFinite(input.data)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.not_finite,
            message: check.message
          });
          status.dirty();
        }
      } else {
        util.assertNever(check);
      }
    }
    return { status: status.value, value: input.data };
  }
  gte(value, message) {
    return this.setLimit("min", value, true, errorUtil.toString(message));
  }
  gt(value, message) {
    return this.setLimit("min", value, false, errorUtil.toString(message));
  }
  lte(value, message) {
    return this.setLimit("max", value, true, errorUtil.toString(message));
  }
  lt(value, message) {
    return this.setLimit("max", value, false, errorUtil.toString(message));
  }
  setLimit(kind, value, inclusive, message) {
    return new ZodNumber({
      ...this._def,
      checks: [
        ...this._def.checks,
        {
          kind,
          value,
          inclusive,
          message: errorUtil.toString(message)
        }
      ]
    });
  }
  _addCheck(check) {
    return new ZodNumber({
      ...this._def,
      checks: [...this._def.checks, check]
    });
  }
  int(message) {
    return this._addCheck({
      kind: "int",
      message: errorUtil.toString(message)
    });
  }
  positive(message) {
    return this._addCheck({
      kind: "min",
      value: 0,
      inclusive: false,
      message: errorUtil.toString(message)
    });
  }
  negative(message) {
    return this._addCheck({
      kind: "max",
      value: 0,
      inclusive: false,
      message: errorUtil.toString(message)
    });
  }
  nonpositive(message) {
    return this._addCheck({
      kind: "max",
      value: 0,
      inclusive: true,
      message: errorUtil.toString(message)
    });
  }
  nonnegative(message) {
    return this._addCheck({
      kind: "min",
      value: 0,
      inclusive: true,
      message: errorUtil.toString(message)
    });
  }
  multipleOf(value, message) {
    return this._addCheck({
      kind: "multipleOf",
      value,
      message: errorUtil.toString(message)
    });
  }
  finite(message) {
    return this._addCheck({
      kind: "finite",
      message: errorUtil.toString(message)
    });
  }
  safe(message) {
    return this._addCheck({
      kind: "min",
      inclusive: true,
      value: Number.MIN_SAFE_INTEGER,
      message: errorUtil.toString(message)
    })._addCheck({
      kind: "max",
      inclusive: true,
      value: Number.MAX_SAFE_INTEGER,
      message: errorUtil.toString(message)
    });
  }
  get minValue() {
    let min = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "min") {
        if (min === null || ch.value > min)
          min = ch.value;
      }
    }
    return min;
  }
  get maxValue() {
    let max = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "max") {
        if (max === null || ch.value < max)
          max = ch.value;
      }
    }
    return max;
  }
  get isInt() {
    return !!this._def.checks.find((ch) => ch.kind === "int" || ch.kind === "multipleOf" && util.isInteger(ch.value));
  }
  get isFinite() {
    let max = null;
    let min = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "finite" || ch.kind === "int" || ch.kind === "multipleOf") {
        return true;
      } else if (ch.kind === "min") {
        if (min === null || ch.value > min)
          min = ch.value;
      } else if (ch.kind === "max") {
        if (max === null || ch.value < max)
          max = ch.value;
      }
    }
    return Number.isFinite(min) && Number.isFinite(max);
  }
}
ZodNumber.create = (params) => {
  return new ZodNumber({
    checks: [],
    typeName: ZodFirstPartyTypeKind.ZodNumber,
    coerce: (params == null ? void 0 : params.coerce) || false,
    ...processCreateParams(params)
  });
};
class ZodBigInt extends ZodType {
  constructor() {
    super(...arguments);
    this.min = this.gte;
    this.max = this.lte;
  }
  _parse(input) {
    if (this._def.coerce) {
      try {
        input.data = BigInt(input.data);
      } catch {
        return this._getInvalidInput(input);
      }
    }
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.bigint) {
      return this._getInvalidInput(input);
    }
    let ctx = void 0;
    const status = new ParseStatus();
    for (const check of this._def.checks) {
      if (check.kind === "min") {
        const tooSmall = check.inclusive ? input.data < check.value : input.data <= check.value;
        if (tooSmall) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_small,
            type: "bigint",
            minimum: check.value,
            inclusive: check.inclusive,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "max") {
        const tooBig = check.inclusive ? input.data > check.value : input.data >= check.value;
        if (tooBig) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_big,
            type: "bigint",
            maximum: check.value,
            inclusive: check.inclusive,
            message: check.message
          });
          status.dirty();
        }
      } else if (check.kind === "multipleOf") {
        if (input.data % check.value !== BigInt(0)) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.not_multiple_of,
            multipleOf: check.value,
            message: check.message
          });
          status.dirty();
        }
      } else {
        util.assertNever(check);
      }
    }
    return { status: status.value, value: input.data };
  }
  _getInvalidInput(input) {
    const ctx = this._getOrReturnCtx(input);
    addIssueToContext(ctx, {
      code: ZodIssueCode.invalid_type,
      expected: ZodParsedType.bigint,
      received: ctx.parsedType
    });
    return INVALID;
  }
  gte(value, message) {
    return this.setLimit("min", value, true, errorUtil.toString(message));
  }
  gt(value, message) {
    return this.setLimit("min", value, false, errorUtil.toString(message));
  }
  lte(value, message) {
    return this.setLimit("max", value, true, errorUtil.toString(message));
  }
  lt(value, message) {
    return this.setLimit("max", value, false, errorUtil.toString(message));
  }
  setLimit(kind, value, inclusive, message) {
    return new ZodBigInt({
      ...this._def,
      checks: [
        ...this._def.checks,
        {
          kind,
          value,
          inclusive,
          message: errorUtil.toString(message)
        }
      ]
    });
  }
  _addCheck(check) {
    return new ZodBigInt({
      ...this._def,
      checks: [...this._def.checks, check]
    });
  }
  positive(message) {
    return this._addCheck({
      kind: "min",
      value: BigInt(0),
      inclusive: false,
      message: errorUtil.toString(message)
    });
  }
  negative(message) {
    return this._addCheck({
      kind: "max",
      value: BigInt(0),
      inclusive: false,
      message: errorUtil.toString(message)
    });
  }
  nonpositive(message) {
    return this._addCheck({
      kind: "max",
      value: BigInt(0),
      inclusive: true,
      message: errorUtil.toString(message)
    });
  }
  nonnegative(message) {
    return this._addCheck({
      kind: "min",
      value: BigInt(0),
      inclusive: true,
      message: errorUtil.toString(message)
    });
  }
  multipleOf(value, message) {
    return this._addCheck({
      kind: "multipleOf",
      value,
      message: errorUtil.toString(message)
    });
  }
  get minValue() {
    let min = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "min") {
        if (min === null || ch.value > min)
          min = ch.value;
      }
    }
    return min;
  }
  get maxValue() {
    let max = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "max") {
        if (max === null || ch.value < max)
          max = ch.value;
      }
    }
    return max;
  }
}
ZodBigInt.create = (params) => {
  return new ZodBigInt({
    checks: [],
    typeName: ZodFirstPartyTypeKind.ZodBigInt,
    coerce: (params == null ? void 0 : params.coerce) ?? false,
    ...processCreateParams(params)
  });
};
class ZodBoolean extends ZodType {
  _parse(input) {
    if (this._def.coerce) {
      input.data = Boolean(input.data);
    }
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.boolean) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.boolean,
        received: ctx.parsedType
      });
      return INVALID;
    }
    return OK(input.data);
  }
}
ZodBoolean.create = (params) => {
  return new ZodBoolean({
    typeName: ZodFirstPartyTypeKind.ZodBoolean,
    coerce: (params == null ? void 0 : params.coerce) || false,
    ...processCreateParams(params)
  });
};
class ZodDate extends ZodType {
  _parse(input) {
    if (this._def.coerce) {
      input.data = new Date(input.data);
    }
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.date) {
      const ctx2 = this._getOrReturnCtx(input);
      addIssueToContext(ctx2, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.date,
        received: ctx2.parsedType
      });
      return INVALID;
    }
    if (Number.isNaN(input.data.getTime())) {
      const ctx2 = this._getOrReturnCtx(input);
      addIssueToContext(ctx2, {
        code: ZodIssueCode.invalid_date
      });
      return INVALID;
    }
    const status = new ParseStatus();
    let ctx = void 0;
    for (const check of this._def.checks) {
      if (check.kind === "min") {
        if (input.data.getTime() < check.value) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_small,
            message: check.message,
            inclusive: true,
            exact: false,
            minimum: check.value,
            type: "date"
          });
          status.dirty();
        }
      } else if (check.kind === "max") {
        if (input.data.getTime() > check.value) {
          ctx = this._getOrReturnCtx(input, ctx);
          addIssueToContext(ctx, {
            code: ZodIssueCode.too_big,
            message: check.message,
            inclusive: true,
            exact: false,
            maximum: check.value,
            type: "date"
          });
          status.dirty();
        }
      } else {
        util.assertNever(check);
      }
    }
    return {
      status: status.value,
      value: new Date(input.data.getTime())
    };
  }
  _addCheck(check) {
    return new ZodDate({
      ...this._def,
      checks: [...this._def.checks, check]
    });
  }
  min(minDate, message) {
    return this._addCheck({
      kind: "min",
      value: minDate.getTime(),
      message: errorUtil.toString(message)
    });
  }
  max(maxDate, message) {
    return this._addCheck({
      kind: "max",
      value: maxDate.getTime(),
      message: errorUtil.toString(message)
    });
  }
  get minDate() {
    let min = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "min") {
        if (min === null || ch.value > min)
          min = ch.value;
      }
    }
    return min != null ? new Date(min) : null;
  }
  get maxDate() {
    let max = null;
    for (const ch of this._def.checks) {
      if (ch.kind === "max") {
        if (max === null || ch.value < max)
          max = ch.value;
      }
    }
    return max != null ? new Date(max) : null;
  }
}
ZodDate.create = (params) => {
  return new ZodDate({
    checks: [],
    coerce: (params == null ? void 0 : params.coerce) || false,
    typeName: ZodFirstPartyTypeKind.ZodDate,
    ...processCreateParams(params)
  });
};
class ZodSymbol extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.symbol) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.symbol,
        received: ctx.parsedType
      });
      return INVALID;
    }
    return OK(input.data);
  }
}
ZodSymbol.create = (params) => {
  return new ZodSymbol({
    typeName: ZodFirstPartyTypeKind.ZodSymbol,
    ...processCreateParams(params)
  });
};
class ZodUndefined extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.undefined) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.undefined,
        received: ctx.parsedType
      });
      return INVALID;
    }
    return OK(input.data);
  }
}
ZodUndefined.create = (params) => {
  return new ZodUndefined({
    typeName: ZodFirstPartyTypeKind.ZodUndefined,
    ...processCreateParams(params)
  });
};
class ZodNull extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.null) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.null,
        received: ctx.parsedType
      });
      return INVALID;
    }
    return OK(input.data);
  }
}
ZodNull.create = (params) => {
  return new ZodNull({
    typeName: ZodFirstPartyTypeKind.ZodNull,
    ...processCreateParams(params)
  });
};
class ZodAny extends ZodType {
  constructor() {
    super(...arguments);
    this._any = true;
  }
  _parse(input) {
    return OK(input.data);
  }
}
ZodAny.create = (params) => {
  return new ZodAny({
    typeName: ZodFirstPartyTypeKind.ZodAny,
    ...processCreateParams(params)
  });
};
class ZodUnknown extends ZodType {
  constructor() {
    super(...arguments);
    this._unknown = true;
  }
  _parse(input) {
    return OK(input.data);
  }
}
ZodUnknown.create = (params) => {
  return new ZodUnknown({
    typeName: ZodFirstPartyTypeKind.ZodUnknown,
    ...processCreateParams(params)
  });
};
class ZodNever extends ZodType {
  _parse(input) {
    const ctx = this._getOrReturnCtx(input);
    addIssueToContext(ctx, {
      code: ZodIssueCode.invalid_type,
      expected: ZodParsedType.never,
      received: ctx.parsedType
    });
    return INVALID;
  }
}
ZodNever.create = (params) => {
  return new ZodNever({
    typeName: ZodFirstPartyTypeKind.ZodNever,
    ...processCreateParams(params)
  });
};
class ZodVoid extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.undefined) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.void,
        received: ctx.parsedType
      });
      return INVALID;
    }
    return OK(input.data);
  }
}
ZodVoid.create = (params) => {
  return new ZodVoid({
    typeName: ZodFirstPartyTypeKind.ZodVoid,
    ...processCreateParams(params)
  });
};
class ZodArray extends ZodType {
  _parse(input) {
    const { ctx, status } = this._processInputParams(input);
    const def = this._def;
    if (ctx.parsedType !== ZodParsedType.array) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.array,
        received: ctx.parsedType
      });
      return INVALID;
    }
    if (def.exactLength !== null) {
      const tooBig = ctx.data.length > def.exactLength.value;
      const tooSmall = ctx.data.length < def.exactLength.value;
      if (tooBig || tooSmall) {
        addIssueToContext(ctx, {
          code: tooBig ? ZodIssueCode.too_big : ZodIssueCode.too_small,
          minimum: tooSmall ? def.exactLength.value : void 0,
          maximum: tooBig ? def.exactLength.value : void 0,
          type: "array",
          inclusive: true,
          exact: true,
          message: def.exactLength.message
        });
        status.dirty();
      }
    }
    if (def.minLength !== null) {
      if (ctx.data.length < def.minLength.value) {
        addIssueToContext(ctx, {
          code: ZodIssueCode.too_small,
          minimum: def.minLength.value,
          type: "array",
          inclusive: true,
          exact: false,
          message: def.minLength.message
        });
        status.dirty();
      }
    }
    if (def.maxLength !== null) {
      if (ctx.data.length > def.maxLength.value) {
        addIssueToContext(ctx, {
          code: ZodIssueCode.too_big,
          maximum: def.maxLength.value,
          type: "array",
          inclusive: true,
          exact: false,
          message: def.maxLength.message
        });
        status.dirty();
      }
    }
    if (ctx.common.async) {
      return Promise.all([...ctx.data].map((item, i) => {
        return def.type._parseAsync(new ParseInputLazyPath(ctx, item, ctx.path, i));
      })).then((result2) => {
        return ParseStatus.mergeArray(status, result2);
      });
    }
    const result = [...ctx.data].map((item, i) => {
      return def.type._parseSync(new ParseInputLazyPath(ctx, item, ctx.path, i));
    });
    return ParseStatus.mergeArray(status, result);
  }
  get element() {
    return this._def.type;
  }
  min(minLength, message) {
    return new ZodArray({
      ...this._def,
      minLength: { value: minLength, message: errorUtil.toString(message) }
    });
  }
  max(maxLength, message) {
    return new ZodArray({
      ...this._def,
      maxLength: { value: maxLength, message: errorUtil.toString(message) }
    });
  }
  length(len, message) {
    return new ZodArray({
      ...this._def,
      exactLength: { value: len, message: errorUtil.toString(message) }
    });
  }
  nonempty(message) {
    return this.min(1, message);
  }
}
ZodArray.create = (schema, params) => {
  return new ZodArray({
    type: schema,
    minLength: null,
    maxLength: null,
    exactLength: null,
    typeName: ZodFirstPartyTypeKind.ZodArray,
    ...processCreateParams(params)
  });
};
function deepPartialify(schema) {
  if (schema instanceof ZodObject) {
    const newShape = {};
    for (const key in schema.shape) {
      const fieldSchema = schema.shape[key];
      newShape[key] = ZodOptional.create(deepPartialify(fieldSchema));
    }
    return new ZodObject({
      ...schema._def,
      shape: () => newShape
    });
  } else if (schema instanceof ZodArray) {
    return new ZodArray({
      ...schema._def,
      type: deepPartialify(schema.element)
    });
  } else if (schema instanceof ZodOptional) {
    return ZodOptional.create(deepPartialify(schema.unwrap()));
  } else if (schema instanceof ZodNullable) {
    return ZodNullable.create(deepPartialify(schema.unwrap()));
  } else if (schema instanceof ZodTuple) {
    return ZodTuple.create(schema.items.map((item) => deepPartialify(item)));
  } else {
    return schema;
  }
}
class ZodObject extends ZodType {
  constructor() {
    super(...arguments);
    this._cached = null;
    this.nonstrict = this.passthrough;
    this.augment = this.extend;
  }
  _getCached() {
    if (this._cached !== null)
      return this._cached;
    const shape = this._def.shape();
    const keys = util.objectKeys(shape);
    this._cached = { shape, keys };
    return this._cached;
  }
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.object) {
      const ctx2 = this._getOrReturnCtx(input);
      addIssueToContext(ctx2, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.object,
        received: ctx2.parsedType
      });
      return INVALID;
    }
    const { status, ctx } = this._processInputParams(input);
    const { shape, keys: shapeKeys } = this._getCached();
    const extraKeys = [];
    if (!(this._def.catchall instanceof ZodNever && this._def.unknownKeys === "strip")) {
      for (const key in ctx.data) {
        if (!shapeKeys.includes(key)) {
          extraKeys.push(key);
        }
      }
    }
    const pairs = [];
    for (const key of shapeKeys) {
      const keyValidator = shape[key];
      const value = ctx.data[key];
      pairs.push({
        key: { status: "valid", value: key },
        value: keyValidator._parse(new ParseInputLazyPath(ctx, value, ctx.path, key)),
        alwaysSet: key in ctx.data
      });
    }
    if (this._def.catchall instanceof ZodNever) {
      const unknownKeys = this._def.unknownKeys;
      if (unknownKeys === "passthrough") {
        for (const key of extraKeys) {
          pairs.push({
            key: { status: "valid", value: key },
            value: { status: "valid", value: ctx.data[key] }
          });
        }
      } else if (unknownKeys === "strict") {
        if (extraKeys.length > 0) {
          addIssueToContext(ctx, {
            code: ZodIssueCode.unrecognized_keys,
            keys: extraKeys
          });
          status.dirty();
        }
      } else if (unknownKeys === "strip") ;
      else {
        throw new Error(`Internal ZodObject error: invalid unknownKeys value.`);
      }
    } else {
      const catchall = this._def.catchall;
      for (const key of extraKeys) {
        const value = ctx.data[key];
        pairs.push({
          key: { status: "valid", value: key },
          value: catchall._parse(
            new ParseInputLazyPath(ctx, value, ctx.path, key)
            //, ctx.child(key), value, getParsedType(value)
          ),
          alwaysSet: key in ctx.data
        });
      }
    }
    if (ctx.common.async) {
      return Promise.resolve().then(async () => {
        const syncPairs = [];
        for (const pair of pairs) {
          const key = await pair.key;
          const value = await pair.value;
          syncPairs.push({
            key,
            value,
            alwaysSet: pair.alwaysSet
          });
        }
        return syncPairs;
      }).then((syncPairs) => {
        return ParseStatus.mergeObjectSync(status, syncPairs);
      });
    } else {
      return ParseStatus.mergeObjectSync(status, pairs);
    }
  }
  get shape() {
    return this._def.shape();
  }
  strict(message) {
    errorUtil.errToObj;
    return new ZodObject({
      ...this._def,
      unknownKeys: "strict",
      ...message !== void 0 ? {
        errorMap: (issue, ctx) => {
          var _a, _b;
          const defaultError = ((_b = (_a = this._def).errorMap) == null ? void 0 : _b.call(_a, issue, ctx).message) ?? ctx.defaultError;
          if (issue.code === "unrecognized_keys")
            return {
              message: errorUtil.errToObj(message).message ?? defaultError
            };
          return {
            message: defaultError
          };
        }
      } : {}
    });
  }
  strip() {
    return new ZodObject({
      ...this._def,
      unknownKeys: "strip"
    });
  }
  passthrough() {
    return new ZodObject({
      ...this._def,
      unknownKeys: "passthrough"
    });
  }
  // const AugmentFactory =
  //   <Def extends ZodObjectDef>(def: Def) =>
  //   <Augmentation extends ZodRawShape>(
  //     augmentation: Augmentation
  //   ): ZodObject<
  //     extendShape<ReturnType<Def["shape"]>, Augmentation>,
  //     Def["unknownKeys"],
  //     Def["catchall"]
  //   > => {
  //     return new ZodObject({
  //       ...def,
  //       shape: () => ({
  //         ...def.shape(),
  //         ...augmentation,
  //       }),
  //     }) as any;
  //   };
  extend(augmentation) {
    return new ZodObject({
      ...this._def,
      shape: () => ({
        ...this._def.shape(),
        ...augmentation
      })
    });
  }
  /**
   * Prior to zod@1.0.12 there was a bug in the
   * inferred type of merged objects. Please
   * upgrade if you are experiencing issues.
   */
  merge(merging) {
    const merged = new ZodObject({
      unknownKeys: merging._def.unknownKeys,
      catchall: merging._def.catchall,
      shape: () => ({
        ...this._def.shape(),
        ...merging._def.shape()
      }),
      typeName: ZodFirstPartyTypeKind.ZodObject
    });
    return merged;
  }
  // merge<
  //   Incoming extends AnyZodObject,
  //   Augmentation extends Incoming["shape"],
  //   NewOutput extends {
  //     [k in keyof Augmentation | keyof Output]: k extends keyof Augmentation
  //       ? Augmentation[k]["_output"]
  //       : k extends keyof Output
  //       ? Output[k]
  //       : never;
  //   },
  //   NewInput extends {
  //     [k in keyof Augmentation | keyof Input]: k extends keyof Augmentation
  //       ? Augmentation[k]["_input"]
  //       : k extends keyof Input
  //       ? Input[k]
  //       : never;
  //   }
  // >(
  //   merging: Incoming
  // ): ZodObject<
  //   extendShape<T, ReturnType<Incoming["_def"]["shape"]>>,
  //   Incoming["_def"]["unknownKeys"],
  //   Incoming["_def"]["catchall"],
  //   NewOutput,
  //   NewInput
  // > {
  //   const merged: any = new ZodObject({
  //     unknownKeys: merging._def.unknownKeys,
  //     catchall: merging._def.catchall,
  //     shape: () =>
  //       objectUtil.mergeShapes(this._def.shape(), merging._def.shape()),
  //     typeName: ZodFirstPartyTypeKind.ZodObject,
  //   }) as any;
  //   return merged;
  // }
  setKey(key, schema) {
    return this.augment({ [key]: schema });
  }
  // merge<Incoming extends AnyZodObject>(
  //   merging: Incoming
  // ): //ZodObject<T & Incoming["_shape"], UnknownKeys, Catchall> = (merging) => {
  // ZodObject<
  //   extendShape<T, ReturnType<Incoming["_def"]["shape"]>>,
  //   Incoming["_def"]["unknownKeys"],
  //   Incoming["_def"]["catchall"]
  // > {
  //   // const mergedShape = objectUtil.mergeShapes(
  //   //   this._def.shape(),
  //   //   merging._def.shape()
  //   // );
  //   const merged: any = new ZodObject({
  //     unknownKeys: merging._def.unknownKeys,
  //     catchall: merging._def.catchall,
  //     shape: () =>
  //       objectUtil.mergeShapes(this._def.shape(), merging._def.shape()),
  //     typeName: ZodFirstPartyTypeKind.ZodObject,
  //   }) as any;
  //   return merged;
  // }
  catchall(index) {
    return new ZodObject({
      ...this._def,
      catchall: index
    });
  }
  pick(mask) {
    const shape = {};
    for (const key of util.objectKeys(mask)) {
      if (mask[key] && this.shape[key]) {
        shape[key] = this.shape[key];
      }
    }
    return new ZodObject({
      ...this._def,
      shape: () => shape
    });
  }
  omit(mask) {
    const shape = {};
    for (const key of util.objectKeys(this.shape)) {
      if (!mask[key]) {
        shape[key] = this.shape[key];
      }
    }
    return new ZodObject({
      ...this._def,
      shape: () => shape
    });
  }
  /**
   * @deprecated
   */
  deepPartial() {
    return deepPartialify(this);
  }
  partial(mask) {
    const newShape = {};
    for (const key of util.objectKeys(this.shape)) {
      const fieldSchema = this.shape[key];
      if (mask && !mask[key]) {
        newShape[key] = fieldSchema;
      } else {
        newShape[key] = fieldSchema.optional();
      }
    }
    return new ZodObject({
      ...this._def,
      shape: () => newShape
    });
  }
  required(mask) {
    const newShape = {};
    for (const key of util.objectKeys(this.shape)) {
      if (mask && !mask[key]) {
        newShape[key] = this.shape[key];
      } else {
        const fieldSchema = this.shape[key];
        let newField = fieldSchema;
        while (newField instanceof ZodOptional) {
          newField = newField._def.innerType;
        }
        newShape[key] = newField;
      }
    }
    return new ZodObject({
      ...this._def,
      shape: () => newShape
    });
  }
  keyof() {
    return createZodEnum(util.objectKeys(this.shape));
  }
}
ZodObject.create = (shape, params) => {
  return new ZodObject({
    shape: () => shape,
    unknownKeys: "strip",
    catchall: ZodNever.create(),
    typeName: ZodFirstPartyTypeKind.ZodObject,
    ...processCreateParams(params)
  });
};
ZodObject.strictCreate = (shape, params) => {
  return new ZodObject({
    shape: () => shape,
    unknownKeys: "strict",
    catchall: ZodNever.create(),
    typeName: ZodFirstPartyTypeKind.ZodObject,
    ...processCreateParams(params)
  });
};
ZodObject.lazycreate = (shape, params) => {
  return new ZodObject({
    shape,
    unknownKeys: "strip",
    catchall: ZodNever.create(),
    typeName: ZodFirstPartyTypeKind.ZodObject,
    ...processCreateParams(params)
  });
};
class ZodUnion extends ZodType {
  _parse(input) {
    const { ctx } = this._processInputParams(input);
    const options = this._def.options;
    function handleResults(results) {
      for (const result of results) {
        if (result.result.status === "valid") {
          return result.result;
        }
      }
      for (const result of results) {
        if (result.result.status === "dirty") {
          ctx.common.issues.push(...result.ctx.common.issues);
          return result.result;
        }
      }
      const unionErrors = results.map((result) => new ZodError(result.ctx.common.issues));
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_union,
        unionErrors
      });
      return INVALID;
    }
    if (ctx.common.async) {
      return Promise.all(options.map(async (option) => {
        const childCtx = {
          ...ctx,
          common: {
            ...ctx.common,
            issues: []
          },
          parent: null
        };
        return {
          result: await option._parseAsync({
            data: ctx.data,
            path: ctx.path,
            parent: childCtx
          }),
          ctx: childCtx
        };
      })).then(handleResults);
    } else {
      let dirty = void 0;
      const issues = [];
      for (const option of options) {
        const childCtx = {
          ...ctx,
          common: {
            ...ctx.common,
            issues: []
          },
          parent: null
        };
        const result = option._parseSync({
          data: ctx.data,
          path: ctx.path,
          parent: childCtx
        });
        if (result.status === "valid") {
          return result;
        } else if (result.status === "dirty" && !dirty) {
          dirty = { result, ctx: childCtx };
        }
        if (childCtx.common.issues.length) {
          issues.push(childCtx.common.issues);
        }
      }
      if (dirty) {
        ctx.common.issues.push(...dirty.ctx.common.issues);
        return dirty.result;
      }
      const unionErrors = issues.map((issues2) => new ZodError(issues2));
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_union,
        unionErrors
      });
      return INVALID;
    }
  }
  get options() {
    return this._def.options;
  }
}
ZodUnion.create = (types, params) => {
  return new ZodUnion({
    options: types,
    typeName: ZodFirstPartyTypeKind.ZodUnion,
    ...processCreateParams(params)
  });
};
function mergeValues(a, b) {
  const aType = getParsedType(a);
  const bType = getParsedType(b);
  if (a === b) {
    return { valid: true, data: a };
  } else if (aType === ZodParsedType.object && bType === ZodParsedType.object) {
    const bKeys = util.objectKeys(b);
    const sharedKeys = util.objectKeys(a).filter((key) => bKeys.indexOf(key) !== -1);
    const newObj = { ...a, ...b };
    for (const key of sharedKeys) {
      const sharedValue = mergeValues(a[key], b[key]);
      if (!sharedValue.valid) {
        return { valid: false };
      }
      newObj[key] = sharedValue.data;
    }
    return { valid: true, data: newObj };
  } else if (aType === ZodParsedType.array && bType === ZodParsedType.array) {
    if (a.length !== b.length) {
      return { valid: false };
    }
    const newArray = [];
    for (let index = 0; index < a.length; index++) {
      const itemA = a[index];
      const itemB = b[index];
      const sharedValue = mergeValues(itemA, itemB);
      if (!sharedValue.valid) {
        return { valid: false };
      }
      newArray.push(sharedValue.data);
    }
    return { valid: true, data: newArray };
  } else if (aType === ZodParsedType.date && bType === ZodParsedType.date && +a === +b) {
    return { valid: true, data: a };
  } else {
    return { valid: false };
  }
}
class ZodIntersection extends ZodType {
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    const handleParsed = (parsedLeft, parsedRight) => {
      if (isAborted(parsedLeft) || isAborted(parsedRight)) {
        return INVALID;
      }
      const merged = mergeValues(parsedLeft.value, parsedRight.value);
      if (!merged.valid) {
        addIssueToContext(ctx, {
          code: ZodIssueCode.invalid_intersection_types
        });
        return INVALID;
      }
      if (isDirty(parsedLeft) || isDirty(parsedRight)) {
        status.dirty();
      }
      return { status: status.value, value: merged.data };
    };
    if (ctx.common.async) {
      return Promise.all([
        this._def.left._parseAsync({
          data: ctx.data,
          path: ctx.path,
          parent: ctx
        }),
        this._def.right._parseAsync({
          data: ctx.data,
          path: ctx.path,
          parent: ctx
        })
      ]).then(([left, right]) => handleParsed(left, right));
    } else {
      return handleParsed(this._def.left._parseSync({
        data: ctx.data,
        path: ctx.path,
        parent: ctx
      }), this._def.right._parseSync({
        data: ctx.data,
        path: ctx.path,
        parent: ctx
      }));
    }
  }
}
ZodIntersection.create = (left, right, params) => {
  return new ZodIntersection({
    left,
    right,
    typeName: ZodFirstPartyTypeKind.ZodIntersection,
    ...processCreateParams(params)
  });
};
class ZodTuple extends ZodType {
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    if (ctx.parsedType !== ZodParsedType.array) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.array,
        received: ctx.parsedType
      });
      return INVALID;
    }
    if (ctx.data.length < this._def.items.length) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.too_small,
        minimum: this._def.items.length,
        inclusive: true,
        exact: false,
        type: "array"
      });
      return INVALID;
    }
    const rest = this._def.rest;
    if (!rest && ctx.data.length > this._def.items.length) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.too_big,
        maximum: this._def.items.length,
        inclusive: true,
        exact: false,
        type: "array"
      });
      status.dirty();
    }
    const items = [...ctx.data].map((item, itemIndex) => {
      const schema = this._def.items[itemIndex] || this._def.rest;
      if (!schema)
        return null;
      return schema._parse(new ParseInputLazyPath(ctx, item, ctx.path, itemIndex));
    }).filter((x) => !!x);
    if (ctx.common.async) {
      return Promise.all(items).then((results) => {
        return ParseStatus.mergeArray(status, results);
      });
    } else {
      return ParseStatus.mergeArray(status, items);
    }
  }
  get items() {
    return this._def.items;
  }
  rest(rest) {
    return new ZodTuple({
      ...this._def,
      rest
    });
  }
}
ZodTuple.create = (schemas, params) => {
  if (!Array.isArray(schemas)) {
    throw new Error("You must pass an array of schemas to z.tuple([ ... ])");
  }
  return new ZodTuple({
    items: schemas,
    typeName: ZodFirstPartyTypeKind.ZodTuple,
    rest: null,
    ...processCreateParams(params)
  });
};
class ZodRecord extends ZodType {
  get keySchema() {
    return this._def.keyType;
  }
  get valueSchema() {
    return this._def.valueType;
  }
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    if (ctx.parsedType !== ZodParsedType.object) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.object,
        received: ctx.parsedType
      });
      return INVALID;
    }
    const pairs = [];
    const keyType = this._def.keyType;
    const valueType = this._def.valueType;
    for (const key in ctx.data) {
      pairs.push({
        key: keyType._parse(new ParseInputLazyPath(ctx, key, ctx.path, key)),
        value: valueType._parse(new ParseInputLazyPath(ctx, ctx.data[key], ctx.path, key)),
        alwaysSet: key in ctx.data
      });
    }
    if (ctx.common.async) {
      return ParseStatus.mergeObjectAsync(status, pairs);
    } else {
      return ParseStatus.mergeObjectSync(status, pairs);
    }
  }
  get element() {
    return this._def.valueType;
  }
  static create(first, second, third) {
    if (second instanceof ZodType) {
      return new ZodRecord({
        keyType: first,
        valueType: second,
        typeName: ZodFirstPartyTypeKind.ZodRecord,
        ...processCreateParams(third)
      });
    }
    return new ZodRecord({
      keyType: ZodString.create(),
      valueType: first,
      typeName: ZodFirstPartyTypeKind.ZodRecord,
      ...processCreateParams(second)
    });
  }
}
class ZodMap extends ZodType {
  get keySchema() {
    return this._def.keyType;
  }
  get valueSchema() {
    return this._def.valueType;
  }
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    if (ctx.parsedType !== ZodParsedType.map) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.map,
        received: ctx.parsedType
      });
      return INVALID;
    }
    const keyType = this._def.keyType;
    const valueType = this._def.valueType;
    const pairs = [...ctx.data.entries()].map(([key, value], index) => {
      return {
        key: keyType._parse(new ParseInputLazyPath(ctx, key, ctx.path, [index, "key"])),
        value: valueType._parse(new ParseInputLazyPath(ctx, value, ctx.path, [index, "value"]))
      };
    });
    if (ctx.common.async) {
      const finalMap = /* @__PURE__ */ new Map();
      return Promise.resolve().then(async () => {
        for (const pair of pairs) {
          const key = await pair.key;
          const value = await pair.value;
          if (key.status === "aborted" || value.status === "aborted") {
            return INVALID;
          }
          if (key.status === "dirty" || value.status === "dirty") {
            status.dirty();
          }
          finalMap.set(key.value, value.value);
        }
        return { status: status.value, value: finalMap };
      });
    } else {
      const finalMap = /* @__PURE__ */ new Map();
      for (const pair of pairs) {
        const key = pair.key;
        const value = pair.value;
        if (key.status === "aborted" || value.status === "aborted") {
          return INVALID;
        }
        if (key.status === "dirty" || value.status === "dirty") {
          status.dirty();
        }
        finalMap.set(key.value, value.value);
      }
      return { status: status.value, value: finalMap };
    }
  }
}
ZodMap.create = (keyType, valueType, params) => {
  return new ZodMap({
    valueType,
    keyType,
    typeName: ZodFirstPartyTypeKind.ZodMap,
    ...processCreateParams(params)
  });
};
class ZodSet extends ZodType {
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    if (ctx.parsedType !== ZodParsedType.set) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.set,
        received: ctx.parsedType
      });
      return INVALID;
    }
    const def = this._def;
    if (def.minSize !== null) {
      if (ctx.data.size < def.minSize.value) {
        addIssueToContext(ctx, {
          code: ZodIssueCode.too_small,
          minimum: def.minSize.value,
          type: "set",
          inclusive: true,
          exact: false,
          message: def.minSize.message
        });
        status.dirty();
      }
    }
    if (def.maxSize !== null) {
      if (ctx.data.size > def.maxSize.value) {
        addIssueToContext(ctx, {
          code: ZodIssueCode.too_big,
          maximum: def.maxSize.value,
          type: "set",
          inclusive: true,
          exact: false,
          message: def.maxSize.message
        });
        status.dirty();
      }
    }
    const valueType = this._def.valueType;
    function finalizeSet(elements2) {
      const parsedSet = /* @__PURE__ */ new Set();
      for (const element of elements2) {
        if (element.status === "aborted")
          return INVALID;
        if (element.status === "dirty")
          status.dirty();
        parsedSet.add(element.value);
      }
      return { status: status.value, value: parsedSet };
    }
    const elements = [...ctx.data.values()].map((item, i) => valueType._parse(new ParseInputLazyPath(ctx, item, ctx.path, i)));
    if (ctx.common.async) {
      return Promise.all(elements).then((elements2) => finalizeSet(elements2));
    } else {
      return finalizeSet(elements);
    }
  }
  min(minSize, message) {
    return new ZodSet({
      ...this._def,
      minSize: { value: minSize, message: errorUtil.toString(message) }
    });
  }
  max(maxSize, message) {
    return new ZodSet({
      ...this._def,
      maxSize: { value: maxSize, message: errorUtil.toString(message) }
    });
  }
  size(size, message) {
    return this.min(size, message).max(size, message);
  }
  nonempty(message) {
    return this.min(1, message);
  }
}
ZodSet.create = (valueType, params) => {
  return new ZodSet({
    valueType,
    minSize: null,
    maxSize: null,
    typeName: ZodFirstPartyTypeKind.ZodSet,
    ...processCreateParams(params)
  });
};
class ZodLazy extends ZodType {
  get schema() {
    return this._def.getter();
  }
  _parse(input) {
    const { ctx } = this._processInputParams(input);
    const lazySchema = this._def.getter();
    return lazySchema._parse({ data: ctx.data, path: ctx.path, parent: ctx });
  }
}
ZodLazy.create = (getter, params) => {
  return new ZodLazy({
    getter,
    typeName: ZodFirstPartyTypeKind.ZodLazy,
    ...processCreateParams(params)
  });
};
class ZodLiteral extends ZodType {
  _parse(input) {
    if (input.data !== this._def.value) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        received: ctx.data,
        code: ZodIssueCode.invalid_literal,
        expected: this._def.value
      });
      return INVALID;
    }
    return { status: "valid", value: input.data };
  }
  get value() {
    return this._def.value;
  }
}
ZodLiteral.create = (value, params) => {
  return new ZodLiteral({
    value,
    typeName: ZodFirstPartyTypeKind.ZodLiteral,
    ...processCreateParams(params)
  });
};
function createZodEnum(values, params) {
  return new ZodEnum({
    values,
    typeName: ZodFirstPartyTypeKind.ZodEnum,
    ...processCreateParams(params)
  });
}
class ZodEnum extends ZodType {
  _parse(input) {
    if (typeof input.data !== "string") {
      const ctx = this._getOrReturnCtx(input);
      const expectedValues = this._def.values;
      addIssueToContext(ctx, {
        expected: util.joinValues(expectedValues),
        received: ctx.parsedType,
        code: ZodIssueCode.invalid_type
      });
      return INVALID;
    }
    if (!this._cache) {
      this._cache = new Set(this._def.values);
    }
    if (!this._cache.has(input.data)) {
      const ctx = this._getOrReturnCtx(input);
      const expectedValues = this._def.values;
      addIssueToContext(ctx, {
        received: ctx.data,
        code: ZodIssueCode.invalid_enum_value,
        options: expectedValues
      });
      return INVALID;
    }
    return OK(input.data);
  }
  get options() {
    return this._def.values;
  }
  get enum() {
    const enumValues = {};
    for (const val of this._def.values) {
      enumValues[val] = val;
    }
    return enumValues;
  }
  get Values() {
    const enumValues = {};
    for (const val of this._def.values) {
      enumValues[val] = val;
    }
    return enumValues;
  }
  get Enum() {
    const enumValues = {};
    for (const val of this._def.values) {
      enumValues[val] = val;
    }
    return enumValues;
  }
  extract(values, newDef = this._def) {
    return ZodEnum.create(values, {
      ...this._def,
      ...newDef
    });
  }
  exclude(values, newDef = this._def) {
    return ZodEnum.create(this.options.filter((opt) => !values.includes(opt)), {
      ...this._def,
      ...newDef
    });
  }
}
ZodEnum.create = createZodEnum;
class ZodNativeEnum extends ZodType {
  _parse(input) {
    const nativeEnumValues = util.getValidEnumValues(this._def.values);
    const ctx = this._getOrReturnCtx(input);
    if (ctx.parsedType !== ZodParsedType.string && ctx.parsedType !== ZodParsedType.number) {
      const expectedValues = util.objectValues(nativeEnumValues);
      addIssueToContext(ctx, {
        expected: util.joinValues(expectedValues),
        received: ctx.parsedType,
        code: ZodIssueCode.invalid_type
      });
      return INVALID;
    }
    if (!this._cache) {
      this._cache = new Set(util.getValidEnumValues(this._def.values));
    }
    if (!this._cache.has(input.data)) {
      const expectedValues = util.objectValues(nativeEnumValues);
      addIssueToContext(ctx, {
        received: ctx.data,
        code: ZodIssueCode.invalid_enum_value,
        options: expectedValues
      });
      return INVALID;
    }
    return OK(input.data);
  }
  get enum() {
    return this._def.values;
  }
}
ZodNativeEnum.create = (values, params) => {
  return new ZodNativeEnum({
    values,
    typeName: ZodFirstPartyTypeKind.ZodNativeEnum,
    ...processCreateParams(params)
  });
};
class ZodPromise extends ZodType {
  unwrap() {
    return this._def.type;
  }
  _parse(input) {
    const { ctx } = this._processInputParams(input);
    if (ctx.parsedType !== ZodParsedType.promise && ctx.common.async === false) {
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.promise,
        received: ctx.parsedType
      });
      return INVALID;
    }
    const promisified = ctx.parsedType === ZodParsedType.promise ? ctx.data : Promise.resolve(ctx.data);
    return OK(promisified.then((data) => {
      return this._def.type.parseAsync(data, {
        path: ctx.path,
        errorMap: ctx.common.contextualErrorMap
      });
    }));
  }
}
ZodPromise.create = (schema, params) => {
  return new ZodPromise({
    type: schema,
    typeName: ZodFirstPartyTypeKind.ZodPromise,
    ...processCreateParams(params)
  });
};
class ZodEffects extends ZodType {
  innerType() {
    return this._def.schema;
  }
  sourceType() {
    return this._def.schema._def.typeName === ZodFirstPartyTypeKind.ZodEffects ? this._def.schema.sourceType() : this._def.schema;
  }
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    const effect = this._def.effect || null;
    const checkCtx = {
      addIssue: (arg) => {
        addIssueToContext(ctx, arg);
        if (arg.fatal) {
          status.abort();
        } else {
          status.dirty();
        }
      },
      get path() {
        return ctx.path;
      }
    };
    checkCtx.addIssue = checkCtx.addIssue.bind(checkCtx);
    if (effect.type === "preprocess") {
      const processed = effect.transform(ctx.data, checkCtx);
      if (ctx.common.async) {
        return Promise.resolve(processed).then(async (processed2) => {
          if (status.value === "aborted")
            return INVALID;
          const result = await this._def.schema._parseAsync({
            data: processed2,
            path: ctx.path,
            parent: ctx
          });
          if (result.status === "aborted")
            return INVALID;
          if (result.status === "dirty")
            return DIRTY(result.value);
          if (status.value === "dirty")
            return DIRTY(result.value);
          return result;
        });
      } else {
        if (status.value === "aborted")
          return INVALID;
        const result = this._def.schema._parseSync({
          data: processed,
          path: ctx.path,
          parent: ctx
        });
        if (result.status === "aborted")
          return INVALID;
        if (result.status === "dirty")
          return DIRTY(result.value);
        if (status.value === "dirty")
          return DIRTY(result.value);
        return result;
      }
    }
    if (effect.type === "refinement") {
      const executeRefinement = (acc) => {
        const result = effect.refinement(acc, checkCtx);
        if (ctx.common.async) {
          return Promise.resolve(result);
        }
        if (result instanceof Promise) {
          throw new Error("Async refinement encountered during synchronous parse operation. Use .parseAsync instead.");
        }
        return acc;
      };
      if (ctx.common.async === false) {
        const inner = this._def.schema._parseSync({
          data: ctx.data,
          path: ctx.path,
          parent: ctx
        });
        if (inner.status === "aborted")
          return INVALID;
        if (inner.status === "dirty")
          status.dirty();
        executeRefinement(inner.value);
        return { status: status.value, value: inner.value };
      } else {
        return this._def.schema._parseAsync({ data: ctx.data, path: ctx.path, parent: ctx }).then((inner) => {
          if (inner.status === "aborted")
            return INVALID;
          if (inner.status === "dirty")
            status.dirty();
          return executeRefinement(inner.value).then(() => {
            return { status: status.value, value: inner.value };
          });
        });
      }
    }
    if (effect.type === "transform") {
      if (ctx.common.async === false) {
        const base = this._def.schema._parseSync({
          data: ctx.data,
          path: ctx.path,
          parent: ctx
        });
        if (!isValid(base))
          return INVALID;
        const result = effect.transform(base.value, checkCtx);
        if (result instanceof Promise) {
          throw new Error(`Asynchronous transform encountered during synchronous parse operation. Use .parseAsync instead.`);
        }
        return { status: status.value, value: result };
      } else {
        return this._def.schema._parseAsync({ data: ctx.data, path: ctx.path, parent: ctx }).then((base) => {
          if (!isValid(base))
            return INVALID;
          return Promise.resolve(effect.transform(base.value, checkCtx)).then((result) => ({
            status: status.value,
            value: result
          }));
        });
      }
    }
    util.assertNever(effect);
  }
}
ZodEffects.create = (schema, effect, params) => {
  return new ZodEffects({
    schema,
    typeName: ZodFirstPartyTypeKind.ZodEffects,
    effect,
    ...processCreateParams(params)
  });
};
ZodEffects.createWithPreprocess = (preprocess, schema, params) => {
  return new ZodEffects({
    schema,
    effect: { type: "preprocess", transform: preprocess },
    typeName: ZodFirstPartyTypeKind.ZodEffects,
    ...processCreateParams(params)
  });
};
class ZodOptional extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType === ZodParsedType.undefined) {
      return OK(void 0);
    }
    return this._def.innerType._parse(input);
  }
  unwrap() {
    return this._def.innerType;
  }
}
ZodOptional.create = (type, params) => {
  return new ZodOptional({
    innerType: type,
    typeName: ZodFirstPartyTypeKind.ZodOptional,
    ...processCreateParams(params)
  });
};
class ZodNullable extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType === ZodParsedType.null) {
      return OK(null);
    }
    return this._def.innerType._parse(input);
  }
  unwrap() {
    return this._def.innerType;
  }
}
ZodNullable.create = (type, params) => {
  return new ZodNullable({
    innerType: type,
    typeName: ZodFirstPartyTypeKind.ZodNullable,
    ...processCreateParams(params)
  });
};
class ZodDefault extends ZodType {
  _parse(input) {
    const { ctx } = this._processInputParams(input);
    let data = ctx.data;
    if (ctx.parsedType === ZodParsedType.undefined) {
      data = this._def.defaultValue();
    }
    return this._def.innerType._parse({
      data,
      path: ctx.path,
      parent: ctx
    });
  }
  removeDefault() {
    return this._def.innerType;
  }
}
ZodDefault.create = (type, params) => {
  return new ZodDefault({
    innerType: type,
    typeName: ZodFirstPartyTypeKind.ZodDefault,
    defaultValue: typeof params.default === "function" ? params.default : () => params.default,
    ...processCreateParams(params)
  });
};
class ZodCatch extends ZodType {
  _parse(input) {
    const { ctx } = this._processInputParams(input);
    const newCtx = {
      ...ctx,
      common: {
        ...ctx.common,
        issues: []
      }
    };
    const result = this._def.innerType._parse({
      data: newCtx.data,
      path: newCtx.path,
      parent: {
        ...newCtx
      }
    });
    if (isAsync(result)) {
      return result.then((result2) => {
        return {
          status: "valid",
          value: result2.status === "valid" ? result2.value : this._def.catchValue({
            get error() {
              return new ZodError(newCtx.common.issues);
            },
            input: newCtx.data
          })
        };
      });
    } else {
      return {
        status: "valid",
        value: result.status === "valid" ? result.value : this._def.catchValue({
          get error() {
            return new ZodError(newCtx.common.issues);
          },
          input: newCtx.data
        })
      };
    }
  }
  removeCatch() {
    return this._def.innerType;
  }
}
ZodCatch.create = (type, params) => {
  return new ZodCatch({
    innerType: type,
    typeName: ZodFirstPartyTypeKind.ZodCatch,
    catchValue: typeof params.catch === "function" ? params.catch : () => params.catch,
    ...processCreateParams(params)
  });
};
class ZodNaN extends ZodType {
  _parse(input) {
    const parsedType = this._getType(input);
    if (parsedType !== ZodParsedType.nan) {
      const ctx = this._getOrReturnCtx(input);
      addIssueToContext(ctx, {
        code: ZodIssueCode.invalid_type,
        expected: ZodParsedType.nan,
        received: ctx.parsedType
      });
      return INVALID;
    }
    return { status: "valid", value: input.data };
  }
}
ZodNaN.create = (params) => {
  return new ZodNaN({
    typeName: ZodFirstPartyTypeKind.ZodNaN,
    ...processCreateParams(params)
  });
};
class ZodBranded extends ZodType {
  _parse(input) {
    const { ctx } = this._processInputParams(input);
    const data = ctx.data;
    return this._def.type._parse({
      data,
      path: ctx.path,
      parent: ctx
    });
  }
  unwrap() {
    return this._def.type;
  }
}
class ZodPipeline extends ZodType {
  _parse(input) {
    const { status, ctx } = this._processInputParams(input);
    if (ctx.common.async) {
      const handleAsync = async () => {
        const inResult = await this._def.in._parseAsync({
          data: ctx.data,
          path: ctx.path,
          parent: ctx
        });
        if (inResult.status === "aborted")
          return INVALID;
        if (inResult.status === "dirty") {
          status.dirty();
          return DIRTY(inResult.value);
        } else {
          return this._def.out._parseAsync({
            data: inResult.value,
            path: ctx.path,
            parent: ctx
          });
        }
      };
      return handleAsync();
    } else {
      const inResult = this._def.in._parseSync({
        data: ctx.data,
        path: ctx.path,
        parent: ctx
      });
      if (inResult.status === "aborted")
        return INVALID;
      if (inResult.status === "dirty") {
        status.dirty();
        return {
          status: "dirty",
          value: inResult.value
        };
      } else {
        return this._def.out._parseSync({
          data: inResult.value,
          path: ctx.path,
          parent: ctx
        });
      }
    }
  }
  static create(a, b) {
    return new ZodPipeline({
      in: a,
      out: b,
      typeName: ZodFirstPartyTypeKind.ZodPipeline
    });
  }
}
class ZodReadonly extends ZodType {
  _parse(input) {
    const result = this._def.innerType._parse(input);
    const freeze = (data) => {
      if (isValid(data)) {
        data.value = Object.freeze(data.value);
      }
      return data;
    };
    return isAsync(result) ? result.then((data) => freeze(data)) : freeze(result);
  }
  unwrap() {
    return this._def.innerType;
  }
}
ZodReadonly.create = (type, params) => {
  return new ZodReadonly({
    innerType: type,
    typeName: ZodFirstPartyTypeKind.ZodReadonly,
    ...processCreateParams(params)
  });
};
var ZodFirstPartyTypeKind;
(function(ZodFirstPartyTypeKind2) {
  ZodFirstPartyTypeKind2["ZodString"] = "ZodString";
  ZodFirstPartyTypeKind2["ZodNumber"] = "ZodNumber";
  ZodFirstPartyTypeKind2["ZodNaN"] = "ZodNaN";
  ZodFirstPartyTypeKind2["ZodBigInt"] = "ZodBigInt";
  ZodFirstPartyTypeKind2["ZodBoolean"] = "ZodBoolean";
  ZodFirstPartyTypeKind2["ZodDate"] = "ZodDate";
  ZodFirstPartyTypeKind2["ZodSymbol"] = "ZodSymbol";
  ZodFirstPartyTypeKind2["ZodUndefined"] = "ZodUndefined";
  ZodFirstPartyTypeKind2["ZodNull"] = "ZodNull";
  ZodFirstPartyTypeKind2["ZodAny"] = "ZodAny";
  ZodFirstPartyTypeKind2["ZodUnknown"] = "ZodUnknown";
  ZodFirstPartyTypeKind2["ZodNever"] = "ZodNever";
  ZodFirstPartyTypeKind2["ZodVoid"] = "ZodVoid";
  ZodFirstPartyTypeKind2["ZodArray"] = "ZodArray";
  ZodFirstPartyTypeKind2["ZodObject"] = "ZodObject";
  ZodFirstPartyTypeKind2["ZodUnion"] = "ZodUnion";
  ZodFirstPartyTypeKind2["ZodDiscriminatedUnion"] = "ZodDiscriminatedUnion";
  ZodFirstPartyTypeKind2["ZodIntersection"] = "ZodIntersection";
  ZodFirstPartyTypeKind2["ZodTuple"] = "ZodTuple";
  ZodFirstPartyTypeKind2["ZodRecord"] = "ZodRecord";
  ZodFirstPartyTypeKind2["ZodMap"] = "ZodMap";
  ZodFirstPartyTypeKind2["ZodSet"] = "ZodSet";
  ZodFirstPartyTypeKind2["ZodFunction"] = "ZodFunction";
  ZodFirstPartyTypeKind2["ZodLazy"] = "ZodLazy";
  ZodFirstPartyTypeKind2["ZodLiteral"] = "ZodLiteral";
  ZodFirstPartyTypeKind2["ZodEnum"] = "ZodEnum";
  ZodFirstPartyTypeKind2["ZodEffects"] = "ZodEffects";
  ZodFirstPartyTypeKind2["ZodNativeEnum"] = "ZodNativeEnum";
  ZodFirstPartyTypeKind2["ZodOptional"] = "ZodOptional";
  ZodFirstPartyTypeKind2["ZodNullable"] = "ZodNullable";
  ZodFirstPartyTypeKind2["ZodDefault"] = "ZodDefault";
  ZodFirstPartyTypeKind2["ZodCatch"] = "ZodCatch";
  ZodFirstPartyTypeKind2["ZodPromise"] = "ZodPromise";
  ZodFirstPartyTypeKind2["ZodBranded"] = "ZodBranded";
  ZodFirstPartyTypeKind2["ZodPipeline"] = "ZodPipeline";
  ZodFirstPartyTypeKind2["ZodReadonly"] = "ZodReadonly";
})(ZodFirstPartyTypeKind || (ZodFirstPartyTypeKind = {}));
const stringType = ZodString.create;
const numberType = ZodNumber.create;
const booleanType = ZodBoolean.create;
const anyType = ZodAny.create;
const unknownType = ZodUnknown.create;
ZodNever.create;
const arrayType = ZodArray.create;
const objectType = ZodObject.create;
const unionType = ZodUnion.create;
ZodIntersection.create;
ZodTuple.create;
const recordType = ZodRecord.create;
const enumType = ZodEnum.create;
ZodPromise.create;
ZodOptional.create;
ZodNullable.create;
const RequestLogSchema = objectType({
  id: stringType(),
  timestamp: stringType(),
  method: stringType(),
  path: stringType(),
  status_code: numberType(),
  response_time_ms: numberType(),
  client_ip: stringType().nullable().optional(),
  user_agent: stringType().nullable().optional(),
  headers: recordType(stringType()).nullable().optional(),
  response_size_bytes: numberType(),
  request_size_bytes: numberType().nullable().optional(),
  error_message: stringType().nullable().optional()
});
objectType({
  timestamp: stringType(),
  status: numberType(),
  method: stringType(),
  url: stringType(),
  responseTime: numberType(),
  size: numberType(),
  status_code: numberType().nullable().optional(),
  response_time_ms: numberType().nullable().optional()
});
const WorkspaceSummarySchema = objectType({
  id: stringType(),
  name: stringType(),
  description: stringType().optional(),
  is_active: booleanType().default(false),
  created_at: stringType().optional(),
  updated_at: stringType().optional(),
  route_count: numberType().optional(),
  fixture_count: numberType().optional()
});
objectType({
  id: stringType(),
  name: stringType(),
  description: stringType().optional(),
  is_active: booleanType().default(false),
  created_at: stringType().optional(),
  updated_at: stringType().optional(),
  fixtures: arrayType(anyType()).optional(),
  routes: arrayType(anyType()).optional()
});
const FixtureInfoSchema = objectType({
  id: stringType(),
  name: stringType(),
  path: stringType(),
  method: stringType().optional(),
  description: stringType().optional(),
  createdAt: stringType(),
  updatedAt: stringType(),
  tags: arrayType(stringType()).optional(),
  content: unionType([stringType(), unknownType()]).optional(),
  version: stringType().optional(),
  size_bytes: numberType().optional(),
  last_modified: stringType().optional(),
  route_path: stringType().optional(),
  protocol: stringType().optional(),
  saved_at: stringType().optional(),
  fingerprint: stringType().optional(),
  metadata: recordType(unknownType()).optional(),
  file_size: numberType().optional(),
  file_path: stringType().optional(),
  size: numberType().optional(),
  created_at: stringType().optional(),
  modified_at: stringType().optional()
});
const ServiceInfoSchema = objectType({
  id: stringType(),
  name: stringType(),
  status: enumType(["active", "inactive", "error"]),
  port: numberType().optional(),
  endpoint: stringType().optional(),
  description: stringType().optional(),
  uptime: numberType().optional(),
  request_count: numberType().optional(),
  error_rate: numberType().optional()
});
const ServerInfoSchema = objectType({
  version: stringType(),
  build_time: stringType(),
  git_sha: stringType(),
  http_server: stringType().nullable().optional(),
  ws_server: stringType().nullable().optional(),
  grpc_server: stringType().nullable().optional(),
  graphql_server: stringType().nullable().optional(),
  api_enabled: booleanType(),
  admin_port: numberType()
});
const DashboardSystemInfoSchema = objectType({
  os: stringType(),
  arch: stringType(),
  uptime: numberType(),
  memory_usage: numberType()
});
const SimpleMetricsDataSchema = objectType({
  total_requests: numberType(),
  active_requests: numberType(),
  average_response_time: numberType(),
  error_rate: numberType()
});
const ServerStatusSchema = objectType({
  server_type: stringType(),
  address: stringType().nullable().optional(),
  running: booleanType(),
  start_time: stringType().nullable().optional(),
  uptime_seconds: numberType().nullable().optional(),
  active_connections: numberType(),
  total_requests: numberType()
});
const SystemInfoSchema = objectType({
  version: stringType(),
  uptime_seconds: numberType(),
  memory_usage_mb: numberType(),
  cpu_usage_percent: numberType(),
  active_threads: numberType(),
  total_routes: numberType(),
  total_fixtures: numberType()
});
const DashboardDataSchema = objectType({
  server_info: ServerInfoSchema,
  system_info: DashboardSystemInfoSchema,
  metrics: SimpleMetricsDataSchema,
  servers: arrayType(ServerStatusSchema),
  recent_logs: arrayType(RequestLogSchema),
  system: SystemInfoSchema
}).passthrough();
objectType({
  service: stringType(),
  route: stringType(),
  avg_response_time: numberType(),
  min_response_time: numberType(),
  max_response_time: numberType(),
  p50_response_time: numberType(),
  p95_response_time: numberType(),
  p99_response_time: numberType(),
  total_requests: numberType(),
  histogram: arrayType(objectType({
    range: stringType(),
    count: numberType()
  })).optional()
});
const WorkspaceListResponseSchema = arrayType(WorkspaceSummarySchema);
const LogsResponseSchema = arrayType(RequestLogSchema);
const DashboardResponseSchema = DashboardDataSchema;
const FixturesResponseSchema = arrayType(FixtureInfoSchema);
arrayType(ServiceInfoSchema);
function safeValidateApiResponse(schema, data) {
  try {
    const result = schema.safeParse(data);
    if (result.success) {
      return { success: true, data: result.data };
    }
    return { success: false, error: result.error };
  } catch (error) {
    logger.error("[VALIDATION ERROR] Exception", error);
    throw error;
  }
}
const API_BASE$5 = "/__mockforge/chains";
const WORKSPACE_API_BASE = "/__mockforge/workspaces";
class ApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async fetchJsonWithValidation(url, schema, options) {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);
    if (!result.success) {
      throw new Error(`API response validation failed: ${result.error.message}`);
    }
    return result.data;
  }
  async listChains() {
    return this.fetchJson(API_BASE$5);
  }
  async getChain(chainId) {
    return this.fetchJson(`${API_BASE$5}/${chainId}`);
  }
  async getGraph() {
    const response = await this.fetchJson("/__mockforge/graph");
    if (response.success && response.data) {
      return response.data;
    }
    return response;
  }
  // State Machine API methods
  async getStateMachines() {
    return this.fetchJson("/__mockforge/api/state-machines");
  }
  async getStateMachine(resourceType) {
    return this.fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`);
  }
  async createStateMachine(stateMachine, visualLayout) {
    return this.fetchJson("/__mockforge/api/state-machines", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ state_machine: stateMachine, visual_layout: visualLayout })
    });
  }
  async updateStateMachine(resourceType, stateMachine, visualLayout) {
    return this.fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ state_machine: stateMachine, visual_layout: visualLayout })
    });
  }
  async deleteStateMachine(resourceType) {
    await this.fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`, {
      method: "DELETE"
    });
  }
  async getStateInstances() {
    return this.fetchJson("/__mockforge/api/state-machines/instances");
  }
  async getStateInstance(resourceId) {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}`);
  }
  async createStateInstance(resourceId, resourceType) {
    return this.fetchJson("/__mockforge/api/state-machines/instances", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ resource_id: resourceId, resource_type: resourceType })
    });
  }
  async executeTransition(resourceId, toState, context) {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/transition`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ resource_id: resourceId, to_state: toState, context })
    });
  }
  async getNextStates(resourceId) {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/next-states`);
  }
  async getCurrentState(resourceId) {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/state`);
  }
  async exportStateMachines() {
    return this.fetchJson("/__mockforge/api/state-machines/export");
  }
  // MockAI OpenAPI Generation API methods
  async generateOpenApiFromTraffic(request) {
    return this.fetchJson("/__mockforge/api/mockai/generate-openapi", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  // MockAI Rule Explanations API methods
  async listRuleExplanations(filters) {
    const params = new URLSearchParams();
    if (filters == null ? void 0 : filters.rule_type) {
      params.append("rule_type", filters.rule_type);
    }
    if ((filters == null ? void 0 : filters.min_confidence) !== void 0) {
      params.append("min_confidence", filters.min_confidence.toString());
    }
    const queryString = params.toString();
    const url = `/__mockforge/api/mockai/rules/explanations${queryString ? `?${queryString}` : ""}`;
    return this.fetchJson(url);
  }
  async getRuleExplanation(ruleId) {
    return this.fetchJson(
      `/__mockforge/api/mockai/rules/${encodeURIComponent(ruleId)}/explanation`
    );
  }
  // MockAI Learn from Examples API method
  async learnFromExamples(request) {
    return this.fetchJson("/__mockforge/api/mockai/learn", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  async importStateMachines(data) {
    await this.fetchJson("/__mockforge/api/state-machines/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data)
    });
  }
  async createChain(definition) {
    return this.fetchJson(API_BASE$5, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ definition })
    });
  }
  async updateChain(chainId, definition) {
    return this.fetchJson(`${API_BASE$5}/${chainId}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ definition })
    });
  }
  async deleteChain(chainId) {
    return this.fetchJson(`${API_BASE$5}/${chainId}`, {
      method: "DELETE"
    });
  }
  async executeChain(chainId, variables) {
    return this.fetchJson(`${API_BASE$5}/${chainId}/execute`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ variables: variables || {} })
    });
  }
  async validateChain(chainId) {
    return this.fetchJson(`${API_BASE$5}/${chainId}/validate`, {
      method: "POST"
    });
  }
  // ==================== WORKSPACE API METHODS ====================
  async listWorkspaces() {
    return this.fetchJsonWithValidation(
      WORKSPACE_API_BASE,
      WorkspaceListResponseSchema
    );
  }
  async getWorkspace(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`);
  }
  async createWorkspace(request) {
    return this.fetchJson(WORKSPACE_API_BASE, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async openWorkspaceFromDirectory(directory) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/open-from-directory`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ directory })
    });
  }
  async deleteWorkspace(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`, {
      method: "DELETE"
    });
  }
  async setActiveWorkspace(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/activate`, {
      method: "POST"
    });
  }
  async getFolder(workspaceId, folderId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders/${folderId}`);
  }
  async createFolder(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async createRequest(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async importToWorkspace(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/import`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async previewImport(request) {
    return this.fetchJson("/__mockforge/import/preview", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async executeRequest(workspaceId, requestId, executionRequest) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/execute`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(executionRequest || {})
    });
  }
  async getRequestHistory(workspaceId, requestId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/history`);
  }
  // ==================== ENVIRONMENT API METHODS ====================
  async getEnvironments(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`);
  }
  async createEnvironment(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  async updateEnvironment(workspaceId, environmentId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  async deleteEnvironment(workspaceId, environmentId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: "DELETE"
    });
  }
  async setActiveEnvironment(workspaceId, environmentId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/activate`, {
      method: "POST"
    });
  }
  async getEnvironmentVariables(workspaceId, environmentId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`);
  }
  async setEnvironmentVariable(workspaceId, environmentId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  async removeEnvironmentVariable(workspaceId, environmentId, variableName) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables/${encodeURIComponent(variableName)}`, {
      method: "DELETE"
    });
  }
  async getAutocompleteSuggestions(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/autocomplete`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  // ==================== ORDERING API METHODS ====================
  async updateWorkspacesOrder(workspaceIds) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/order`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ workspace_ids: workspaceIds })
    });
  }
  async updateEnvironmentsOrder(workspaceId, environmentIds) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/order`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ environment_ids: environmentIds })
    });
  }
  // ==================== SYNC API METHODS ====================
  async getSyncStatus(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/status`);
  }
  async configureSync(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/configure`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  async disableSync(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/disable`, {
      method: "POST"
    });
  }
  async triggerSync(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/trigger`, {
      method: "POST"
    });
  }
  async getSyncChanges(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/changes`);
  }
  async confirmSyncChanges(workspaceId, request) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/confirm`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  // ==================== ENCRYPTION API METHODS ====================
  async getWorkspaceEncryptionStatus(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/status`);
  }
  async getWorkspaceEncryptionConfig(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`);
  }
  async enableWorkspaceEncryption(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/enable`, {
      method: "POST"
    });
  }
  async disableWorkspaceEncryption(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/disable`, {
      method: "POST"
    });
  }
  async checkWorkspaceSecurity(workspaceId) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/security-check`, {
      method: "POST"
    });
  }
  async exportWorkspaceEncrypted(workspaceId, exportPath) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/export`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ export_path: exportPath })
    });
  }
  async importWorkspaceEncrypted(importPath, workspaceId, backupKey) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/import`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ import_path: importPath, backup_key: backupKey })
    });
  }
  async updateWorkspaceEncryptionConfig(workspaceId, config) {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
}
class ImportApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async importPostman(request) {
    return this.fetchJson("/__mockforge/import/postman", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async importInsomnia(request) {
    return this.fetchJson("/__mockforge/import/insomnia", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async importCurl(request) {
    return this.fetchJson("/__mockforge/import/curl", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async importOpenApi(request) {
    return this.fetchJson("/__mockforge/import/openapi", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async previewImport(request) {
    return this.fetchJson("/__mockforge/import/preview", {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(request)
    });
  }
  async getImportHistory() {
    return this.fetchJson("/__mockforge/import/history");
  }
  async clearImportHistory() {
    return this.fetchJson("/__mockforge/import/history/clear", {
      method: "POST"
    });
  }
}
class FixturesApiService {
  constructor() {
    this.getFixtures = this.getFixtures.bind(this);
    this.deleteFixture = this.deleteFixture.bind(this);
    this.deleteFixturesBulk = this.deleteFixturesBulk.bind(this);
    this.downloadFixture = this.downloadFixture.bind(this);
    this.renameFixture = this.renameFixture.bind(this);
    this.moveFixture = this.moveFixture.bind(this);
  }
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async fetchJsonWithValidation(url, schema, options) {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);
    if (!result.success) {
      throw new Error(`API response validation failed: ${result.error.message}`);
    }
    return result.data;
  }
  async getFixtures() {
    return this.fetchJsonWithValidation(
      "/__mockforge/fixtures",
      FixturesResponseSchema
    );
  }
  async deleteFixture(fixtureId) {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}`, {
      method: "DELETE"
    });
  }
  async deleteFixturesBulk(fixtureIds) {
    return this.fetchJson("/__mockforge/fixtures/bulk", {
      method: "DELETE",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ fixture_ids: fixtureIds })
    });
  }
  async downloadFixture(fixtureId) {
    const response = await authenticatedFetch(`/__mockforge/fixtures/${fixtureId}/download`);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.blob();
  }
  async renameFixture(fixtureId, newName) {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}/rename`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ new_name: newName })
    });
  }
  async moveFixture(fixtureId, newPath) {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}/move`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ new_path: newPath })
    });
  }
}
class DashboardApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async fetchJsonWithValidation(url, schema, options) {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);
    if (!result.success) {
      throw new Error(`API response validation failed: ${result.error.message}`);
    }
    return result.data;
  }
  async getDashboard() {
    return this.fetchJsonWithValidation(
      "/__mockforge/dashboard",
      DashboardResponseSchema
    );
  }
  async getHealth() {
    return this.fetchJson("/__mockforge/health");
  }
}
class ServerApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getServerInfo() {
    return this.fetchJson("/__mockforge/server-info");
  }
  async restartServer(reason) {
    return this.fetchJson("/__mockforge/servers/restart", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reason: reason || "Manual restart" })
    });
  }
  async getRestartStatus() {
    return this.fetchJson("/__mockforge/servers/restart/status");
  }
}
class RoutesApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getRoutes() {
    return this.fetchJson("/__mockforge/routes");
  }
}
class LogsApiService {
  constructor() {
    this.getLogs = this.getLogs.bind(this);
    this.clearLogs = this.clearLogs.bind(this);
  }
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async fetchJsonWithValidation(url, schema, options) {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);
    if (!result.success) {
      throw new Error(`API response validation failed: ${result.error.message}`);
    }
    return result.data;
  }
  async getLogs(params) {
    let url = "/__mockforge/logs";
    if (params && Object.keys(params).length > 0) {
      const stringParams = {};
      for (const [key, value] of Object.entries(params)) {
        if (value !== void 0 && value !== null) {
          stringParams[key] = String(value);
        }
      }
      if (Object.keys(stringParams).length > 0) {
        const queryString = "?" + new URLSearchParams(stringParams).toString();
        url = `/__mockforge/logs${queryString}`;
      }
    }
    return this.fetchJsonWithValidation(url, LogsResponseSchema);
  }
  async clearLogs() {
    return this.fetchJson("/__mockforge/logs", {
      method: "DELETE"
    });
  }
}
class MetricsApiService {
  constructor() {
    this.getMetrics = this.getMetrics.bind(this);
  }
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getMetrics() {
    return this.fetchJson("/__mockforge/metrics");
  }
}
class ConfigApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getConfig() {
    return this.fetchJson("/__mockforge/config");
  }
  async updateLatency(config) {
    return this.fetchJson("/__mockforge/config/latency", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
  async updateFaults(config) {
    return this.fetchJson("/__mockforge/config/faults", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
  async updateProxy(config) {
    return this.fetchJson("/__mockforge/config/proxy", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
}
class ValidationApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getValidation() {
    return this.fetchJson("/__mockforge/validation");
  }
  async updateValidation(config) {
    return this.fetchJson("/__mockforge/validation", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
}
class EnvApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getEnvVars() {
    return this.fetchJson("/__mockforge/env");
  }
  async updateEnvVar(key, value) {
    return this.fetchJson("/__mockforge/env", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key, value })
    });
  }
}
class FilesApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getFileContent(request) {
    return this.fetchJson("/__mockforge/files/content", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  async saveFileContent(request) {
    return this.fetchJson("/__mockforge/files/save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
}
class SmokeTestsApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getSmokeTests() {
    return this.fetchJson("/__mockforge/smoke");
  }
  async runSmokeTests() {
    return this.fetchJson("/__mockforge/smoke/run", {
      method: "GET"
    });
  }
}
class ChaosApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  /**
   * Get current chaos configuration
   */
  async getChaosConfig() {
    return this.fetchJson("/api/chaos/config");
  }
  /**
   * Get current chaos status
   */
  async getChaosStatus() {
    return this.fetchJson("/api/chaos/status");
  }
  /**
   * Update latency configuration
   */
  async updateChaosLatency(config) {
    return this.fetchJson("/api/chaos/config/latency", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
  /**
   * Update fault injection configuration
   */
  async updateChaosFaults(config) {
    return this.fetchJson("/api/chaos/config/faults", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
  /**
   * Update traffic shaping configuration
   */
  async updateChaosTraffic(config) {
    return this.fetchJson("/api/chaos/config/traffic", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config)
    });
  }
  /**
   * Enable chaos engineering
   */
  async enableChaos() {
    return this.fetchJson("/api/chaos/enable", {
      method: "POST"
    });
  }
  /**
   * Disable chaos engineering
   */
  async disableChaos() {
    return this.fetchJson("/api/chaos/disable", {
      method: "POST"
    });
  }
  /**
   * Reset chaos configuration to defaults
   */
  async resetChaos() {
    return this.fetchJson("/api/chaos/reset", {
      method: "POST"
    });
  }
  /**
   * Get latency metrics (time-series data)
   */
  async getLatencyMetrics() {
    return this.fetchJson("/api/chaos/metrics/latency");
  }
  /**
   * Get latency statistics
   */
  async getLatencyStats() {
    return this.fetchJson("/api/chaos/metrics/latency/stats");
  }
  /**
   * List all network profiles (built-in + custom)
   */
  async getNetworkProfiles() {
    return this.fetchJson("/api/chaos/profiles");
  }
  /**
   * Get a specific network profile by name
   */
  async getNetworkProfile(name) {
    return this.fetchJson(`/api/chaos/profiles/${encodeURIComponent(name)}`);
  }
  /**
   * Apply a network profile
   */
  async applyNetworkProfile(name) {
    return this.fetchJson(`/api/chaos/profiles/${encodeURIComponent(name)}/apply`, {
      method: "POST"
    });
  }
  /**
   * Create a custom network profile
   */
  async createNetworkProfile(profile) {
    return this.fetchJson("/api/chaos/profiles", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(profile)
    });
  }
  /**
   * Delete a custom network profile
   */
  async deleteNetworkProfile(name) {
    return this.fetchJson(`/api/chaos/profiles/${encodeURIComponent(name)}`, {
      method: "DELETE"
    });
  }
  /**
   * Export a network profile (JSON or YAML)
   */
  async exportNetworkProfile(name, format = "json") {
    const response = await authenticatedFetch(`/api/chaos/profiles/${encodeURIComponent(name)}/export?format=${format}`);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    if (format === "yaml") {
      return response.text();
    }
    return response.json();
  }
  /**
   * Import a network profile from JSON or YAML
   */
  async importNetworkProfile(content, format) {
    return this.fetchJson("/api/chaos/profiles/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content, format })
    });
  }
  /**
   * Update error pattern configuration
   */
  async updateErrorPattern(pattern) {
    const currentConfig = await this.getChaosConfig();
    const faultConfig = currentConfig.fault_injection || {};
    faultConfig.error_pattern = pattern;
    return this.updateChaosFaults(faultConfig);
  }
}
class TimeTravelApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      const error = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
      throw new Error(error.error || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  // Time Travel Status
  async getStatus() {
    return this.fetchJson("/__mockforge/time-travel/status");
  }
  async enable(time, scale) {
    return this.fetchJson("/__mockforge/time-travel/enable", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ time, scale })
    });
  }
  async disable() {
    return this.fetchJson("/__mockforge/time-travel/disable", {
      method: "POST"
    });
  }
  async advance(duration) {
    return this.fetchJson("/__mockforge/time-travel/advance", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ duration })
    });
  }
  async setTime(time) {
    return this.fetchJson("/__mockforge/time-travel/set", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ time })
    });
  }
  async setScale(scale) {
    return this.fetchJson("/__mockforge/time-travel/scale", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ scale })
    });
  }
  async reset() {
    return this.fetchJson("/__mockforge/time-travel/reset", {
      method: "POST"
    });
  }
  // Cron Jobs
  async listCronJobs() {
    return this.fetchJson("/__mockforge/time-travel/cron");
  }
  async getCronJob(id) {
    return this.fetchJson(`/__mockforge/time-travel/cron/${id}`);
  }
  async createCronJob(job) {
    return this.fetchJson("/__mockforge/time-travel/cron", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(job)
    });
  }
  async deleteCronJob(id) {
    return this.fetchJson(`/__mockforge/time-travel/cron/${id}`, {
      method: "DELETE"
    });
  }
  async setCronJobEnabled(id, enabled) {
    return this.fetchJson(`/__mockforge/time-travel/cron/${id}/enable`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled })
    });
  }
  // Mutation Rules
  async listMutationRules() {
    return this.fetchJson("/__mockforge/time-travel/mutations");
  }
  async getMutationRule(id) {
    return this.fetchJson(`/__mockforge/time-travel/mutations/${id}`);
  }
  async createMutationRule(rule) {
    return this.fetchJson("/__mockforge/time-travel/mutations", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(rule)
    });
  }
  async deleteMutationRule(id) {
    return this.fetchJson(`/__mockforge/time-travel/mutations/${id}`, {
      method: "DELETE"
    });
  }
  async setMutationRuleEnabled(id, enabled) {
    return this.fetchJson(`/__mockforge/time-travel/mutations/${id}/enable`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled })
    });
  }
}
class RealityApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      const error = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
      throw new Error(error.error || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  /**
   * Get current reality level and configuration
   */
  async getRealityLevel() {
    return this.fetchJson("/__mockforge/reality/level");
  }
  /**
   * Set reality level (1-5)
   */
  async setRealityLevel(level) {
    return this.fetchJson("/__mockforge/reality/level", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ level })
    });
  }
  /**
   * List all available reality presets
   */
  async listPresets() {
    return this.fetchJson("/__mockforge/reality/presets");
  }
  /**
   * Import a reality preset
   */
  async importPreset(path) {
    return this.fetchJson("/__mockforge/reality/presets/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path })
    });
  }
  /**
   * Export current reality configuration as a preset
   */
  async exportPreset(name, description) {
    return this.fetchJson("/__mockforge/reality/presets/export", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, description })
    });
  }
}
class PluginsApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getPlugins(params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.type) queryParams.append("type", params.type);
    if (params == null ? void 0 : params.status) queryParams.append("status", params.status);
    const queryString = queryParams.toString() ? `?${queryParams.toString()}` : "";
    return this.fetchJson(`/__mockforge/plugins${queryString}`);
  }
  async getPluginStatus() {
    return this.fetchJson("/__mockforge/plugins/status");
  }
  async getPluginDetails(pluginId) {
    return this.fetchJson(`/__mockforge/plugins/${pluginId}`);
  }
  async deletePlugin(pluginId) {
    return this.fetchJson(`/__mockforge/plugins/${pluginId}`, {
      method: "DELETE"
    });
  }
  async reloadPlugin(pluginId) {
    return this.fetchJson("/__mockforge/plugins/reload", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ plugin_id: pluginId })
    });
  }
  async reloadAllPlugins() {
    const { plugins } = await this.getPlugins();
    const results = await Promise.allSettled(
      plugins.map((plugin) => this.reloadPlugin(plugin.id))
    );
    const failed = results.filter((r) => r.status === "rejected").length;
    if (failed > 0) {
      throw new Error(`Failed to reload ${failed} plugin(s)`);
    }
    return { message: `Successfully reloaded ${plugins.length} plugin(s)` };
  }
}
class VerificationApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      const errorData = await response.json().catch(() => ({}));
      throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }
  async verify(pattern, expected) {
    return this.fetchJson("/__mockforge/verification/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ pattern, expected })
    });
  }
  async count(pattern) {
    return this.fetchJson("/__mockforge/verification/count", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ pattern })
    });
  }
  async verifySequence(patterns) {
    return this.fetchJson("/__mockforge/verification/sequence", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ patterns })
    });
  }
  async verifyNever(pattern) {
    return this.fetchJson("/__mockforge/verification/never", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(pattern)
    });
  }
  async verifyAtLeast(pattern, min) {
    return this.fetchJson("/__mockforge/verification/at-least", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ pattern, min })
    });
  }
}
class ContractDiffApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      const errorText = await response.text();
      let errorMessage = `HTTP error! status: ${response.status}`;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorMessage;
      } catch {
      }
      throw new Error(errorMessage);
    }
    const json = await response.json();
    return json.data || json;
  }
  async uploadRequest(request) {
    const response = await authenticatedFetch("/__mockforge/contract-diff/upload", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
    const json = await response.json();
    if (!response.ok) {
      throw new Error(json.error || `HTTP error! status: ${response.status}`);
    }
    return json;
  }
  async getCapturedRequests(params) {
    const queryParams = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== void 0) {
          queryParams.append(key, String(value));
        }
      });
    }
    const url = `/__mockforge/contract-diff/captures${queryParams.toString() ? `?${queryParams}` : ""}`;
    return this.fetchJson(url);
  }
  async getCapturedRequest(id) {
    return this.fetchJson(`/__mockforge/contract-diff/captures/${id}`);
  }
  async analyzeCapturedRequest(id, payload) {
    return this.fetchJson(`/__mockforge/contract-diff/captures/${id}/analyze`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });
  }
  async getStatistics() {
    return this.fetchJson("/__mockforge/contract-diff/statistics");
  }
  async generatePatchFile(id, payload) {
    return this.fetchJson(`/__mockforge/contract-diff/captures/${id}/patch`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });
  }
}
class ProxyApiService {
  async fetchJson(url, options) {
    const response = await fetch(url, options);
    if (!response.ok) {
      const errorText = await response.text();
      let errorMessage = `HTTP error! status: ${response.status}`;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorMessage;
      } catch {
      }
      throw new Error(errorMessage);
    }
    const json = await response.json();
    return json.data || json;
  }
  async getProxyRules() {
    return this.fetchJson("/__mockforge/api/proxy/rules");
  }
  async getProxyRule(id) {
    return this.fetchJson(`/__mockforge/api/proxy/rules/${id}`);
  }
  async createProxyRule(rule) {
    return this.fetchJson("/__mockforge/api/proxy/rules", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(rule)
    });
  }
  async updateProxyRule(id, rule) {
    return this.fetchJson(`/__mockforge/api/proxy/rules/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(rule)
    });
  }
  async deleteProxyRule(id) {
    return this.fetchJson(`/__mockforge/api/proxy/rules/${id}`, {
      method: "DELETE"
    });
  }
  async getProxyInspect(limit) {
    const url = limit ? `/__mockforge/api/proxy/inspect?limit=${limit}` : "/__mockforge/api/proxy/inspect";
    return this.fetchJson(url);
  }
  // ==================== PLAYGROUND API METHODS ====================
  /**
   * List available endpoints for playground
   */
  async listPlaygroundEndpoints(workspaceId) {
    const url = workspaceId ? `/?workspace_id=${encodeURIComponent(workspaceId)}` : "";
    return this.fetchJson(`/__mockforge/playground/endpoints${url}`);
  }
  /**
   * Execute a REST request
   */
  async executeRestRequest(request) {
    return this.fetchJson("/__mockforge/playground/execute", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * Execute a GraphQL query
   */
  async executeGraphQLQuery(request) {
    return this.fetchJson("/__mockforge/playground/graphql", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * Perform GraphQL introspection
   */
  async graphQLIntrospect() {
    return this.fetchJson("/__mockforge/playground/graphql/introspect");
  }
  /**
   * Get request history
   */
  async getPlaygroundHistory(params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.limit) {
      queryParams.append("limit", params.limit.toString());
    }
    if (params == null ? void 0 : params.protocol) {
      queryParams.append("protocol", params.protocol);
    }
    if (params == null ? void 0 : params.workspace_id) {
      queryParams.append("workspace_id", params.workspace_id);
    }
    const url = queryParams.toString() ? `/__mockforge/playground/history?${queryParams.toString()}` : "/__mockforge/playground/history";
    return this.fetchJson(url);
  }
  /**
   * Replay a request from history
   */
  async replayRequest(requestId) {
    return this.fetchJson(`/__mockforge/playground/history/${requestId}/replay`, {
      method: "POST"
    });
  }
  /**
   * Generate code snippets
   */
  async generateCodeSnippet(request) {
    return this.fetchJson("/__mockforge/playground/snippets", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  // ==================== BEHAVIORAL CLONING API METHODS ====================
  /**
   * List all recorded flows
   */
  async getFlows(params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.limit) {
      queryParams.append("limit", params.limit.toString());
    }
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = queryParams.toString() ? `/__mockforge/flows?${queryParams.toString()}` : "/__mockforge/flows";
    const response = await this.fetchJson(url);
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to fetch flows");
    }
    return response.data;
  }
  /**
   * Get flow details with timeline
   */
  async getFlow(flowId, params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = queryParams.toString() ? `/__mockforge/flows/${flowId}?${queryParams.toString()}` : `/__mockforge/flows/${flowId}`;
    const response = await this.fetchJson(url);
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to fetch flow");
    }
    return response.data;
  }
  /**
   * Tag a flow
   */
  async tagFlow(flowId, request, params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = queryParams.toString() ? `/__mockforge/flows/${flowId}/tag?${queryParams.toString()}` : `/__mockforge/flows/${flowId}/tag`;
    const response = await this.fetchJson(url, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to tag flow");
    }
    return response.data;
  }
  /**
   * Compile flow to scenario
   */
  async compileFlow(flowId, request, params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = queryParams.toString() ? `/__mockforge/flows/${flowId}/compile?${queryParams.toString()}` : `/__mockforge/flows/${flowId}/compile`;
    const response = await this.fetchJson(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to compile flow");
    }
    return response.data;
  }
  /**
   * List all scenarios
   */
  async getScenarios(params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.limit) {
      queryParams.append("limit", params.limit.toString());
    }
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = queryParams.toString() ? `/__mockforge/scenarios?${queryParams.toString()}` : "/__mockforge/scenarios";
    const response = await this.fetchJson(url);
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to fetch scenarios");
    }
    return response.data;
  }
  /**
   * Get scenario details
   */
  async getScenario(scenarioId, params) {
    const queryParams = new URLSearchParams();
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = queryParams.toString() ? `/__mockforge/scenarios/${scenarioId}?${queryParams.toString()}` : `/__mockforge/scenarios/${scenarioId}`;
    const response = await this.fetchJson(url);
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to fetch scenario");
    }
    return response.data;
  }
  /**
   * Export scenario
   */
  async exportScenario(scenarioId, format = "yaml", params) {
    const queryParams = new URLSearchParams();
    queryParams.append("format", format);
    if (params == null ? void 0 : params.db_path) {
      queryParams.append("db_path", params.db_path);
    }
    const url = `/__mockforge/scenarios/${scenarioId}/export?${queryParams.toString()}`;
    const response = await this.fetchJson(url);
    if (!response.success || !response.data) {
      throw new Error(response.error || "Failed to export scenario");
    }
    return response.data.content;
  }
}
const apiService = new ApiService();
const importApi = new ImportApiService();
const fixturesApi = new FixturesApiService();
const proxyApi = new ProxyApiService();
const dashboardApi = new DashboardApiService();
const serverApi = new ServerApiService();
const routesApi = new RoutesApiService();
const logsApi = new LogsApiService();
const metricsApi = new MetricsApiService();
const configApi = new ConfigApiService();
const validationApi = new ValidationApiService();
const envApi = new EnvApiService();
const filesApi = new FilesApiService();
const smokeTestsApi = new SmokeTestsApiService();
const pluginsApi = new PluginsApiService();
const chaosApi = new ChaosApiService();
const timeTravelApi = new TimeTravelApiService();
const realityApi = new RealityApiService();
const verificationApi = new VerificationApiService();
const contractDiffApi = new ContractDiffApiService();
logger.info("API Services initialized", {
  apiService: !!apiService,
  importApi: !!importApi,
  fixturesApi: !!fixturesApi,
  fixturesApiGetFixtures: typeof (fixturesApi == null ? void 0 : fixturesApi.getFixtures),
  dashboardApi: !!dashboardApi,
  serverApi: !!serverApi,
  routesApi: !!routesApi,
  logsApi: !!logsApi,
  metricsApi: !!metricsApi,
  configApi: !!configApi,
  validationApi: !!validationApi,
  envApi: !!envApi,
  filesApi: !!filesApi,
  smokeTestsApi: !!smokeTestsApi,
  pluginsApi: !!pluginsApi,
  chaosApi: !!chaosApi,
  timeTravelApi: !!timeTravelApi
});
//! Drift Budget and Incident Management API
//!
//! This module provides API client functions for drift budget and incident management.
class DriftBudgetApiService {
  async fetchJson(url, options) {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error("Authentication required");
      }
      if (response.status === 403) {
        throw new Error("Access denied");
      }
      const errorText = await response.text();
      let errorMessage = `HTTP error! status: ${response.status}`;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorMessage;
      } catch {
      }
      throw new Error(errorMessage);
    }
    const json = await response.json();
    return json.data || json;
  }
  /**
   * Create or update a drift budget
   * POST /api/v1/drift/budgets
   */
  async createOrUpdateBudget(request) {
    return this.fetchJson("/api/v1/drift/budgets", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * List drift budgets
   * GET /api/v1/drift/budgets
   */
  async listBudgets(params) {
    const queryParams = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== void 0) {
          queryParams.append(key, String(value));
        }
      });
    }
    const url = `/api/v1/drift/budgets${queryParams.toString() ? `?${queryParams}` : ""}`;
    return this.fetchJson(url);
  }
  /**
   * Get a specific drift budget
   * GET /api/v1/drift/budgets/{id}
   */
  async getBudget(id) {
    return this.fetchJson(`/api/v1/drift/budgets/${id}`);
  }
  /**
   * List incidents
   * GET /api/v1/drift/incidents
   */
  async listIncidents(params) {
    const queryParams = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== void 0) {
          queryParams.append(key, String(value));
        }
      });
    }
    const url = `/api/v1/drift/incidents${queryParams.toString() ? `?${queryParams}` : ""}`;
    return this.fetchJson(url);
  }
  /**
   * Get a specific incident
   * GET /api/v1/drift/incidents/{id}
   */
  async getIncident(id) {
    return this.fetchJson(`/api/v1/drift/incidents/${id}`);
  }
  /**
   * Update an incident
   * PATCH /api/v1/drift/incidents/{id}
   */
  async updateIncident(id, request) {
    return this.fetchJson(`/api/v1/drift/incidents/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * Resolve an incident
   * POST /api/v1/drift/incidents/{id}/resolve
   */
  async resolveIncident(id) {
    return this.fetchJson(`/api/v1/drift/incidents/${id}/resolve`, {
      method: "POST"
    });
  }
  /**
   * Get incident statistics
   * GET /api/v1/drift/incidents/stats
   */
  async getIncidentStatistics() {
    return this.fetchJson("/api/v1/drift/incidents/stats");
  }
  /**
   * List fitness functions
   * GET /api/v1/drift/fitness-functions
   */
  async listFitnessFunctions() {
    return this.fetchJson("/api/v1/drift/fitness-functions");
  }
  /**
   * Get a specific fitness function
   * GET /api/v1/drift/fitness-functions/{id}
   */
  async getFitnessFunction(id) {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}`);
  }
  /**
   * Create a fitness function
   * POST /api/v1/drift/fitness-functions
   */
  async createFitnessFunction(request) {
    return this.fetchJson("/api/v1/drift/fitness-functions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * Update a fitness function
   * PATCH /api/v1/drift/fitness-functions/{id}
   */
  async updateFitnessFunction(id, request) {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * Delete a fitness function
   * DELETE /api/v1/drift/fitness-functions/{id}
   */
  async deleteFitnessFunction(id) {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}`, {
      method: "DELETE"
    });
  }
  /**
   * Test a fitness function
   * POST /api/v1/drift/fitness-functions/{id}/test
   */
  async testFitnessFunction(id, request) {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}/test`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * List consumer mappings
   * GET /api/v1/drift/consumer-mappings
   */
  async listConsumerMappings() {
    return this.fetchJson("/api/v1/drift/consumer-mappings");
  }
  /**
   * Get consumer mapping for a specific endpoint
   * GET /api/v1/drift/consumer-mappings/lookup?endpoint=...&method=...
   */
  async getConsumerMapping(endpoint, method) {
    const queryParams = new URLSearchParams({ endpoint, method });
    return this.fetchJson(`/api/v1/drift/consumer-mappings/lookup?${queryParams}`);
  }
  /**
   * Create or update a consumer mapping
   * POST /api/v1/drift/consumer-mappings
   */
  async createConsumerMapping(request) {
    return this.fetchJson("/api/v1/drift/consumer-mappings", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request)
    });
  }
  /**
   * Get consumer impact for an incident
   * GET /api/v1/drift/incidents/{id}/impact
   */
  async getIncidentImpact(id) {
    return this.fetchJson(`/api/v1/drift/incidents/${id}/impact`);
  }
}
const driftApi = new DriftBudgetApiService();
const queryKeys = {
  dashboard: ["dashboard"],
  health: ["health"],
  serverInfo: ["serverInfo"],
  restartStatus: ["restartStatus"],
  routes: ["routes"],
  logs: ["logs"],
  metrics: ["metrics"],
  config: ["config"],
  validation: ["validation"],
  envVars: ["envVars"],
  fixtures: ["fixtures"],
  smokeTests: ["smokeTests"],
  import: ["import"],
  importHistory: ["importHistory"],
  chaosConfig: ["chaosConfig"],
  chaosStatus: ["chaosStatus"],
  chaosLatencyMetrics: ["chaosLatencyMetrics"],
  chaosLatencyStats: ["chaosLatencyStats"],
  networkProfiles: ["networkProfiles"],
  networkProfile: (name) => ["networkProfile", name],
  timeTravelStatus: ["timeTravelStatus"],
  cronJobs: ["cronJobs"],
  mutationRules: ["mutationRules"],
  proxyRules: ["proxyRules"],
  proxyInspect: ["proxyInspect"],
  realityLevel: ["realityLevel"],
  realityPresets: ["realityPresets"],
  lifecyclePresets: ["lifecyclePresets"],
  lifecyclePreset: (name) => ["lifecyclePreset", name],
  // Drift budget and incidents
  driftBudgets: ["driftBudgets"],
  driftBudget: (id) => ["driftBudget", id],
  driftIncidents: (params) => ["driftIncidents", params],
  driftIncident: (id) => ["driftIncident", id],
  driftIncidentStats: ["driftIncidentStats"]
};
function useDashboard() {
  return useQuery({
    queryKey: queryKeys.dashboard,
    queryFn: async () => {
      if (!dashboardApi) {
        logger.error("dashboardApi is undefined!");
        throw new Error("dashboardApi service not initialized");
      }
      return dashboardApi.getDashboard();
    },
    refetchInterval: 5e3,
    // Refetch every 5 seconds for real-time updates
    refetchIntervalInBackground: true,
    // Continue refetching even when tab is in background
    staleTime: 2e3
    // Consider data stale after 2 seconds
  });
}
function useServerInfo() {
  return useQuery({
    queryKey: queryKeys.serverInfo,
    queryFn: serverApi.getServerInfo,
    staleTime: 3e4
  });
}
function useRestartStatus() {
  return useQuery({
    queryKey: queryKeys.restartStatus,
    queryFn: serverApi.getRestartStatus,
    refetchInterval: 5e3,
    // Poll frequently during restart
    enabled: false
    // Only enable when restart is initiated
  });
}
function useRestartServers() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (reason) => serverApi.restartServer(reason),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.restartStatus });
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useLogs(params) {
  const { refetchInterval, ...apiParams } = params || {};
  return useQuery({
    queryKey: [...queryKeys.logs, apiParams],
    queryFn: () => logsApi.getLogs(apiParams),
    staleTime: 5e3,
    // Logs can change frequently
    refetchInterval
    // Optional auto-refetch interval
  });
}
function useClearLogs() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: logsApi.clearLogs,
    onSuccess: () => {
      queryClient2.setQueryData(queryKeys.logs, []);
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useMetrics() {
  return useQuery({
    queryKey: queryKeys.metrics,
    queryFn: async () => {
      if (!metricsApi) {
        logger.error("metricsApi is undefined!");
        throw new Error("metricsApi service not initialized");
      }
      return metricsApi.getMetrics();
    },
    refetchInterval: 15e3,
    // Update metrics every 15 seconds
    staleTime: 5e3
  });
}
function useConfig() {
  return useQuery({
    queryKey: queryKeys.config,
    queryFn: configApi.getConfig,
    staleTime: 3e4
  });
}
function useUpdateLatency() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: configApi.updateLatency,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.config });
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useUpdateFaults() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: configApi.updateFaults,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.config });
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useUpdateProxy() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: configApi.updateProxy,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.config });
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useValidation() {
  return useQuery({
    queryKey: queryKeys.validation,
    queryFn: validationApi.getValidation,
    staleTime: 3e4
  });
}
function useUpdateValidation() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: validationApi.updateValidation,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.validation });
    }
  });
}
function useFixtures() {
  return useQuery({
    queryKey: ["fixtures-v2"],
    queryFn: async () => {
      try {
        const response = await fetch("/__mockforge/fixtures");
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        return Array.isArray(data.data) ? data.data : [];
      } catch (error) {
        logger.error("[FIXTURES ERROR] Failed to fetch fixtures", error);
        throw error;
      }
    },
    retry: false,
    staleTime: 3e4
  });
}
function useImportPostman() {
  return useMutation({
    mutationFn: importApi.importPostman
  });
}
function useImportInsomnia() {
  return useMutation({
    mutationFn: importApi.importInsomnia
  });
}
function useImportCurl() {
  return useMutation({
    mutationFn: importApi.importCurl
  });
}
function usePreviewImport() {
  return useMutation({
    mutationFn: importApi.previewImport
  });
}
function useImportHistory() {
  return useQuery({
    queryKey: queryKeys.importHistory,
    queryFn: importApi.getImportHistory,
    staleTime: 3e4
    // Import history doesn't change often
  });
}
function useClearImportHistory() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: importApi.clearImportHistory,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.importHistory });
    }
  });
}
function useEnvironments(workspaceId) {
  return useQuery({
    queryKey: ["environments", workspaceId],
    queryFn: () => apiService.getEnvironments(workspaceId),
    enabled: !!workspaceId,
    staleTime: 1e4
    // Cache for 10 seconds
  });
}
function useCreateEnvironment(workspaceId) {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (request) => apiService.createEnvironment(workspaceId, request),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["environments", workspaceId] });
    }
  });
}
function useUpdateEnvironment(workspaceId, environmentId) {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (request) => apiService.updateEnvironment(workspaceId, environmentId, request),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["environments", workspaceId] });
      queryClient2.invalidateQueries({ queryKey: ["environment-variables", workspaceId, environmentId] });
    }
  });
}
function useDeleteEnvironment(workspaceId) {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (environmentId) => apiService.deleteEnvironment(workspaceId, environmentId),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["environments", workspaceId] });
    }
  });
}
function useSetActiveEnvironment(workspaceId) {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (environmentId) => apiService.setActiveEnvironment(workspaceId, environmentId),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["environments", workspaceId] });
      queryClient2.invalidateQueries({ queryKey: ["workspaces"] });
    }
  });
}
function useEnvironmentVariables(workspaceId, environmentId) {
  return useQuery({
    queryKey: ["environment-variables", workspaceId, environmentId],
    queryFn: () => apiService.getEnvironmentVariables(workspaceId, environmentId),
    enabled: !!workspaceId && !!environmentId,
    staleTime: 5e3
    // Cache for 5 seconds
  });
}
function useAutocomplete(workspaceId) {
  return useMutation({
    mutationFn: (request) => apiService.getAutocompleteSuggestions(workspaceId, request)
  });
}
function useUpdateWorkspacesOrder() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (workspaceIds) => apiService.updateWorkspacesOrder(workspaceIds),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["workspaces"] });
    }
  });
}
function useUpdateEnvironmentsOrder(workspaceId) {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (environmentIds) => apiService.updateEnvironmentsOrder(workspaceId, environmentIds),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["environments", workspaceId] });
    }
  });
}
function useChaosConfig() {
  return useQuery({
    queryKey: queryKeys.chaosConfig,
    queryFn: () => chaosApi.getChaosConfig(),
    staleTime: 1e4,
    // Consider data stale after 10 seconds
    refetchInterval: 3e4
    // Refetch every 30 seconds
  });
}
function useChaosStatus() {
  return useQuery({
    queryKey: queryKeys.chaosStatus,
    queryFn: () => chaosApi.getChaosStatus(),
    staleTime: 5e3,
    // Consider data stale after 5 seconds
    refetchInterval: 1e4
    // Refetch every 10 seconds
  });
}
function useUpdateChaosLatency() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (config) => chaosApi.updateChaosLatency(config),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    }
  });
}
function useUpdateChaosFaults() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (config) => chaosApi.updateChaosFaults(config),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    }
  });
}
function useUpdateChaosTraffic() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (config) => chaosApi.updateChaosTraffic(config),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    }
  });
}
function useResetChaos() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: () => chaosApi.resetChaos(),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    }
  });
}
function useChaosLatencyMetrics() {
  return useQuery({
    queryKey: queryKeys.chaosLatencyMetrics,
    queryFn: () => chaosApi.getLatencyMetrics(),
    refetchInterval: 500,
    // Refetch every 500ms for real-time graph
    staleTime: 100
  });
}
function useChaosLatencyStats() {
  return useQuery({
    queryKey: queryKeys.chaosLatencyStats,
    queryFn: () => chaosApi.getLatencyStats(),
    refetchInterval: 2e3,
    // Refetch every 2 seconds
    staleTime: 500
  });
}
function useNetworkProfiles() {
  return useQuery({
    queryKey: queryKeys.networkProfiles,
    queryFn: () => chaosApi.getNetworkProfiles(),
    staleTime: 3e4,
    // Consider data stale after 30 seconds
    refetchInterval: 6e4
    // Refetch every minute
  });
}
function useApplyNetworkProfile() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (name) => chaosApi.applyNetworkProfile(name),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosStatus });
      queryClient2.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    }
  });
}
function useCreateNetworkProfile() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (profile) => chaosApi.createNetworkProfile(profile),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    }
  });
}
function useDeleteNetworkProfile() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (name) => chaosApi.deleteNetworkProfile(name),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    }
  });
}
function useExportNetworkProfile() {
  return useMutation({
    mutationFn: ({ name, format }) => chaosApi.exportNetworkProfile(name, format || "json")
  });
}
function useImportNetworkProfile() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: ({ content, format }) => chaosApi.importNetworkProfile(content, format),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    }
  });
}
function useUpdateErrorPattern() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (pattern) => chaosApi.updateErrorPattern(pattern),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient2.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    }
  });
}
function useDriftIncidents(params, options) {
  return useQuery({
    queryKey: queryKeys.driftIncidents(params),
    queryFn: () => driftApi.listIncidents(params),
    refetchInterval: (options == null ? void 0 : options.refetchInterval) || 5e3,
    // Auto-refresh every 5 seconds by default
    staleTime: 2e3
  });
}
function useUpdateDriftIncident() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: ({ id, request }) => driftApi.updateIncident(id, request),
    onSuccess: (_, variables) => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.driftIncident(variables.id) });
      queryClient2.invalidateQueries({ queryKey: queryKeys.driftIncidents() });
      queryClient2.invalidateQueries({ queryKey: queryKeys.driftIncidentStats });
    }
  });
}
function useResolveDriftIncident() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (id) => driftApi.resolveIncident(id),
    onSuccess: (_, id) => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.driftIncident(id) });
      queryClient2.invalidateQueries({ queryKey: queryKeys.driftIncidents() });
      queryClient2.invalidateQueries({ queryKey: queryKeys.driftIncidentStats });
    }
  });
}
function useDriftIncidentStatistics() {
  return useQuery({
    queryKey: queryKeys.driftIncidentStats,
    queryFn: () => driftApi.getIncidentStatistics(),
    refetchInterval: 1e4,
    // Refetch every 10 seconds
    staleTime: 5e3
  });
}
function useTimeTravelStatus() {
  return useQuery({
    queryKey: queryKeys.timeTravelStatus,
    queryFn: () => timeTravelApi.getStatus(),
    refetchInterval: 2e3,
    // Refetch every 2 seconds for real-time updates
    staleTime: 1e3
  });
}
function useUpdatePersonaLifecycles() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: async (workspace = "default") => {
      const response = await fetch(`/api/v1/consistency/persona/update-lifecycles?workspace=${workspace}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" }
      });
      if (response.status === 405) {
        console.debug("[TimeTravel] Persona lifecycle update endpoint not available (405)");
        return null;
      }
      if (!response.ok) {
        throw new Error(`Failed to update persona lifecycles: ${response.status}`);
      }
      return response.json();
    },
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["consistency", "state"] });
      queryClient2.invalidateQueries({ queryKey: ["consistency", "persona"] });
    },
    onError: (error) => {
      var _a;
      if (!((_a = error.message) == null ? void 0 : _a.includes("405"))) {
        console.warn("[TimeTravel] Failed to update persona lifecycles:", error);
      }
    }
  });
}
function useLivePreviewLifecycleUpdates(workspace = "default", enabled = true) {
  const { data: timeStatus } = useTimeTravelStatus();
  const updateLifecycles = useUpdatePersonaLifecycles();
  const previousTimeRef = React.useRef();
  React.useEffect(() => {
    if (!enabled || !(timeStatus == null ? void 0 : timeStatus.enabled)) {
      return;
    }
    const currentTime = timeStatus.current_time;
    if (currentTime && currentTime !== previousTimeRef.current) {
      previousTimeRef.current = currentTime;
      updateLifecycles.mutate(workspace, {
        onSuccess: () => {
        },
        onError: () => {
        }
      });
    }
  }, [timeStatus == null ? void 0 : timeStatus.current_time, timeStatus == null ? void 0 : timeStatus.enabled, enabled, workspace, updateLifecycles]);
}
function useEnableTimeTravel() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: ({ time, scale }) => timeTravelApi.enable(time, scale),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    }
  });
}
function useDisableTimeTravel() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: () => timeTravelApi.disable(),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    }
  });
}
function useAdvanceTime() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (duration) => timeTravelApi.advance(duration),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    }
  });
}
function useSetTime() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (time) => timeTravelApi.setTime(time),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    }
  });
}
function useSetTimeScale() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (scale) => timeTravelApi.setScale(scale),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    }
  });
}
function useResetTimeTravel() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: () => timeTravelApi.reset(),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    }
  });
}
function useCronJobs() {
  return useQuery({
    queryKey: queryKeys.cronJobs,
    queryFn: () => timeTravelApi.listCronJobs(),
    refetchInterval: 5e3,
    // Refetch every 5 seconds
    staleTime: 2e3
  });
}
function useMutationRules() {
  return useQuery({
    queryKey: queryKeys.mutationRules,
    queryFn: () => timeTravelApi.listMutationRules(),
    refetchInterval: 5e3,
    // Refetch every 5 seconds
    staleTime: 2e3
  });
}
function useProxyRules() {
  return useQuery({
    queryKey: queryKeys.proxyRules,
    queryFn: () => proxyApi.getProxyRules(),
    staleTime: 1e4,
    // Cache for 10 seconds
    refetchInterval: 5e3
    // Auto-refresh every 5 seconds
  });
}
function useCreateProxyRule() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (rule) => proxyApi.createProxyRule(rule),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.proxyRules });
    }
  });
}
function useUpdateProxyRule() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: ({ id, rule }) => proxyApi.updateProxyRule(id, rule),
    onSuccess: (_, variables) => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.proxyRules });
      queryClient2.invalidateQueries({ queryKey: [...queryKeys.proxyRules, variables.id] });
    }
  });
}
function useDeleteProxyRule() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (id) => proxyApi.deleteProxyRule(id),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.proxyRules });
    }
  });
}
function useProxyInspect(limit) {
  return useQuery({
    queryKey: [...queryKeys.proxyInspect, limit],
    queryFn: () => proxyApi.getProxyInspect(limit),
    staleTime: 2e3,
    // Very short cache for real-time inspection
    refetchInterval: 2e3
    // Auto-refresh every 2 seconds
  });
}
function useRealityLevel() {
  return useQuery({
    queryKey: queryKeys.realityLevel,
    queryFn: () => realityApi.getRealityLevel(),
    staleTime: 1e4,
    // Consider data stale after 10 seconds
    refetchInterval: 3e4
    // Refetch every 30 seconds
  });
}
function useSetRealityLevel() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (level) => realityApi.setRealityLevel(level),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.realityLevel });
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useRealityPresets() {
  return useQuery({
    queryKey: queryKeys.realityPresets,
    queryFn: () => realityApi.listPresets(),
    staleTime: 6e4
    // Presets don't change often
  });
}
function useImportRealityPreset() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: (path) => realityApi.importPreset(path),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.realityLevel });
      queryClient2.invalidateQueries({ queryKey: queryKeys.realityPresets });
      queryClient2.invalidateQueries({ queryKey: queryKeys.dashboard });
    }
  });
}
function useExportRealityPreset() {
  const queryClient2 = useQueryClient();
  return useMutation({
    mutationFn: ({ name, description }) => realityApi.exportPreset(name, description),
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: queryKeys.realityPresets });
    }
  });
}
function usePrefetch() {
  const queryClient2 = useQueryClient();
  const prefetchDashboard = reactExports.useCallback(() => {
    queryClient2.prefetchQuery({
      queryKey: queryKeys.dashboard,
      staleTime: 5 * 60 * 1e3
      // 5 minutes
    });
  }, [queryClient2]);
  const prefetchMetrics = reactExports.useCallback(() => {
    queryClient2.prefetchQuery({
      queryKey: queryKeys.metrics,
      staleTime: 2 * 60 * 1e3
      // 2 minutes
    });
  }, [queryClient2]);
  const prefetchLogs = reactExports.useCallback(() => {
    queryClient2.prefetchQuery({
      queryKey: [...queryKeys.logs],
      staleTime: 1 * 60 * 1e3
      // 1 minute
    });
  }, [queryClient2]);
  const prefetchConfig = reactExports.useCallback(() => {
    queryClient2.prefetchQuery({
      queryKey: queryKeys.config,
      staleTime: 10 * 60 * 1e3
      // 10 minutes (config changes rarely)
    });
  }, [queryClient2]);
  const prefetchAll = reactExports.useCallback(() => {
    prefetchDashboard();
    prefetchMetrics();
    prefetchLogs();
    prefetchConfig();
  }, [prefetchDashboard, prefetchMetrics, prefetchLogs, prefetchConfig]);
  return {
    prefetchDashboard,
    prefetchMetrics,
    prefetchLogs,
    prefetchConfig,
    prefetchAll
  };
}
function useStartupPrefetch() {
  const { prefetchAll } = usePrefetch();
  reactExports.useEffect(() => {
    const timer = setTimeout(() => {
      prefetchAll();
    }, 1e3);
    return () => clearTimeout(timer);
  }, [prefetchAll]);
}
const useWorkspaceStore = create()(
  persist(
    (set, get) => ({
      activeWorkspace: null,
      workspaces: [],
      loading: false,
      error: null,
      setActiveWorkspace: (workspace) => {
        set({ activeWorkspace: workspace });
      },
      loadWorkspaces: async () => {
        set({ loading: true, error: null });
        try {
          const workspaces = await apiService.listWorkspaces();
          set({ workspaces, loading: false });
          const activeWorkspace = workspaces.find((w) => w.is_active);
          if (activeWorkspace) {
            set({ activeWorkspace });
          }
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : "Failed to load workspaces",
            loading: false,
            workspaces: []
          });
        }
      },
      setActiveWorkspaceById: async (workspaceId) => {
        set({ loading: true, error: null });
        try {
          await apiService.setActiveWorkspace(workspaceId);
          const workspaces = await apiService.listWorkspaces();
          set({ workspaces, loading: false });
          const activeWorkspace = workspaces.find((w) => w.is_active);
          set({ activeWorkspace });
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : "Failed to set active workspace",
            loading: false,
            workspaces: []
          });
        }
      },
      refreshWorkspaces: async () => {
        await get().loadWorkspaces();
      }
    }),
    {
      name: "mockforge-workspace",
      partialize: (state) => ({
        activeWorkspace: state.activeWorkspace
      })
    }
  )
);
const VirtualBackendsPage = () => {
  const [activeTab, setActiveTab] = reactExports.useState("entities");
  const entities = [
    { name: "users", recordCount: 150, columns: 8, lastModified: "2 mins ago" },
    { name: "orders", recordCount: 1240, columns: 12, lastModified: "Just now" },
    { name: "products", recordCount: 56, columns: 6, lastModified: "1 hour ago" },
    { name: "payments", recordCount: 890, columns: 10, lastModified: "5 mins ago" }
  ];
  const snapshots = [
    { id: "snap_1", name: "Pre-Migration Backup", timestamp: "2023-10-25 10:00 AM", description: "Before applying v2 schema changes", size: "1.2 MB" },
    { id: "snap_2", name: "Clean State", timestamp: "2023-10-24 09:00 AM", description: "Fresh database with seed data only", size: "0.5 MB" }
  ];
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 max-w-7xl mx-auto h-full flex flex-col", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-start mb-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 mb-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100", children: "Virtual Backend" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "px-2 py-0.5 rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 text-xs font-medium border border-green-200 dark:border-green-900/50", children: "Running" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-gray-600 dark:text-gray-400", children: "Manage your stateful mock database, entities, and time-travel snapshots." })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("button", { className: "flex items-center px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Clock, { className: "w-4 h-4 mr-2" }),
          "Simulate Time"
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("button", { className: "flex items-center px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "w-4 h-4 mr-2" }),
          "New Entity"
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex border-b border-gray-200 dark:border-gray-700 mb-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setActiveTab("entities"),
          className: `px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === "entities" ? "border-blue-600 text-blue-600 dark:text-blue-400" : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"}`,
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Database, { className: "w-4 h-4" }),
            "Entities & Schema"
          ]
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setActiveTab("data"),
          className: `px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === "data" ? "border-blue-600 text-blue-600 dark:text-blue-400" : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"}`,
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Table, { className: "w-4 h-4" }),
            "Data Explorer"
          ]
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setActiveTab("snapshots"),
          className: `px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === "snapshots" ? "border-blue-600 text-blue-600 dark:text-blue-400" : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"}`,
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(History, { className: "w-4 h-4" }),
            "Snapshots & Time Travel"
          ]
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setActiveTab("settings"),
          className: `px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === "settings" ? "border-blue-600 text-blue-600 dark:text-blue-400" : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"}`,
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "w-4 h-4" }),
            "Configuration"
          ]
        }
      )
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex-1 bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden", children: [
      activeTab === "entities" && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-0", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("table", { className: "w-full text-left text-sm", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("thead", { className: "bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("tr", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Entity Name" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Records" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Columns" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Last Modified" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right", children: "Actions" })
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("tbody", { className: "divide-y divide-gray-200 dark:divide-gray-700", children: entities.map((entity) => /* @__PURE__ */ jsxRuntimeExports.jsxs("tr", { className: "hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-2 bg-blue-50 dark:bg-blue-900/20 rounded text-blue-600 dark:text-blue-400", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Table, { className: "w-4 h-4" }) }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium text-gray-900 dark:text-gray-100", children: entity.name })
          ] }) }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-gray-600 dark:text-gray-300", children: entity.recordCount.toLocaleString() }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-gray-600 dark:text-gray-300", children: entity.columns }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-gray-500 dark:text-gray-400", children: entity.lastModified }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-right", children: /* @__PURE__ */ jsxRuntimeExports.jsx("button", { className: "text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 font-medium text-sm", children: "View Data" }) })
        ] }, entity.name)) })
      ] }) }),
      activeTab === "data" && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-col h-full", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 border-b border-gray-200 dark:border-gray-700 flex gap-4 items-center bg-gray-50 dark:bg-gray-900/30", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative flex-1 max-w-md", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Search, { className: "absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "input",
              {
                type: "text",
                placeholder: "Search records...",
                className: "w-full pl-9 pr-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none"
              }
            )
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-2 ml-auto", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("button", { className: "px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700", children: "Filter" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("button", { className: "px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700", children: "Export JSON" })
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex-1 flex items-center justify-center text-gray-500 dark:text-gray-400", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Table, { className: "w-12 h-12 mx-auto mb-3 opacity-20" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { children: "Select an entity to view records" })
        ] }) })
      ] }),
      activeTab === "snapshots" && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-center mb-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-medium text-gray-900 dark:text-gray-100", children: "Database Snapshots" }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("button", { className: "flex items-center px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-colors", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Save, { className: "w-4 h-4 mr-2" }),
            "Create Snapshot"
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid gap-4", children: snapshots.map((snap) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-300 dark:hover:border-blue-700 transition-colors bg-white dark:bg-gray-800", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start gap-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-3 bg-purple-50 dark:bg-purple-900/20 rounded-lg text-purple-600 dark:text-purple-400", children: /* @__PURE__ */ jsxRuntimeExports.jsx(History, { className: "w-6 h-6" }) }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100", children: snap.name }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500 dark:text-gray-400 mt-1", children: snap.description }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-4 mt-2 text-xs text-gray-400", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: snap.timestamp }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "•" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: snap.size })
              ] })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("button", { className: "p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors", title: "Restore", children: /* @__PURE__ */ jsxRuntimeExports.jsx(RotateCcw, { className: "w-5 h-5" }) }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("button", { className: "p-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors", children: /* @__PURE__ */ jsxRuntimeExports.jsx(EllipsisVertical, { className: "w-5 h-5" }) })
          ] })
        ] }, snap.id)) })
      ] }),
      activeTab === "settings" && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "max-w-2xl", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-4", children: "Engine Configuration" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Storage Backend" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("select", { className: "w-full p-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-md", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { children: "SQLite (Persistent)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { children: "In-Memory (Fast)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { children: "JSON File (Portable)" })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Auto-Snapshot Interval" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("select", { className: "w-full p-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-md", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { children: "Disabled" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { children: "Every 1 hour" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { children: "Every 24 hours" })
            ] })
          ] })
        ] })
      ] }) })
    ] })
  ] });
};
const TunnelsPage = () => {
  const [tunnels, setTunnels] = reactExports.useState([
    {
      id: "tun_123",
      name: "Payment Service Dev",
      local_port: 8080,
      public_url: "https://payment-dev.mockforge.io",
      status: "active",
      created_at: (/* @__PURE__ */ new Date()).toISOString(),
      region: "us-east-1"
    }
  ]);
  const [isCreateModalOpen, setIsCreateModalOpen] = reactExports.useState(false);
  const [newTunnel, setNewTunnel] = reactExports.useState({ name: "", port: "8080" });
  const handleCreate = () => {
    const tunnel = {
      id: `tun_${Math.random().toString(36).substr(2, 9)}`,
      name: newTunnel.name,
      local_port: parseInt(newTunnel.port),
      public_url: `https://${newTunnel.name.toLowerCase().replace(/\s+/g, "-")}.mockforge.io`,
      status: "active",
      created_at: (/* @__PURE__ */ new Date()).toISOString(),
      region: "us-east-1"
    };
    setTunnels([...tunnels, tunnel]);
    setIsCreateModalOpen(false);
    setNewTunnel({ name: "", port: "8080" });
  };
  const handleDelete = (id) => {
    setTunnels(tunnels.filter((t) => t.id !== id));
  };
  const copyToClipboard = (text) => {
    navigator.clipboard.writeText(text);
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 max-w-7xl mx-auto", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-start mb-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2", children: "Tunnels" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-gray-600 dark:text-gray-400", children: "Expose your local mock servers to the internet via secure tunnels." })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setIsCreateModalOpen(true),
          className: "flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "w-4 h-4 mr-2" }),
            "Start Tunnel"
          ]
        }
      )
    ] }),
    tunnels.length === 0 ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Globe, { className: "w-16 h-16 mx-auto text-gray-400 mb-4" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-2", children: "No Active Tunnels" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-gray-500 dark:text-gray-400 mb-6", children: "Create a tunnel to share your local mocks with external services or teammates." }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        "button",
        {
          onClick: () => setIsCreateModalOpen(true),
          className: "px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors",
          children: "Create First Tunnel"
        }
      )
    ] }) : /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "overflow-x-auto", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("table", { className: "w-full text-left text-sm", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("thead", { className: "bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("tr", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Name" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Status" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Local Port" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Public URL" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400", children: "Region" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("th", { className: "px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right", children: "Actions" })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("tbody", { className: "divide-y divide-gray-200 dark:divide-gray-700", children: tunnels.map((tunnel) => /* @__PURE__ */ jsxRuntimeExports.jsxs("tr", { className: "hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("td", { className: "px-6 py-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium text-gray-900 dark:text-gray-100", children: tunnel.name }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xs text-gray-500 font-mono mt-0.5", children: tunnel.id })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: `inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${tunnel.status === "active" ? "bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30" : "bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700"}`, children: [
          tunnel.status === "active" ? /* @__PURE__ */ jsxRuntimeExports.jsx(Wifi, { className: "w-3 h-3 mr-1" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(WifiOff, { className: "w-3 h-3 mr-1" }),
          tunnel.status
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-gray-600 dark:text-gray-300 font-mono", children: tunnel.local_port }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-mono text-gray-600 dark:text-gray-300 truncate max-w-[200px]", children: tunnel.public_url }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "button",
            {
              onClick: () => copyToClipboard(tunnel.public_url),
              className: "p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded transition-colors",
              title: "Copy URL",
              children: /* @__PURE__ */ jsxRuntimeExports.jsx(Copy, { className: "w-3.5 h-3.5" })
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "a",
            {
              href: tunnel.public_url,
              target: "_blank",
              rel: "noopener noreferrer",
              className: "p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded transition-colors",
              title: "Open URL",
              children: /* @__PURE__ */ jsxRuntimeExports.jsx(ExternalLink, { className: "w-3.5 h-3.5" })
            }
          )
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-gray-600 dark:text-gray-300", children: tunnel.region }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("td", { className: "px-6 py-4 text-right", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: () => handleDelete(tunnel.id),
            className: "p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors",
            title: "Stop Tunnel",
            children: /* @__PURE__ */ jsxRuntimeExports.jsx(Trash2, { className: "w-4 h-4" })
          }
        ) })
      ] }, tunnel.id)) })
    ] }) }) }),
    isCreateModalOpen && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-6 border-b border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-xl font-semibold text-gray-900 dark:text-gray-100", children: "Start New Tunnel" }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 p-4 rounded-lg text-sm", children: "This will create a secure tunnel from a public URL to your local machine." }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300", children: "Tunnel Name" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "input",
            {
              type: "text",
              value: newTunnel.name,
              onChange: (e) => setNewTunnel({ ...newTunnel, name: e.target.value }),
              placeholder: "e.g., My Payment Mock",
              className: "w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300", children: "Local Port" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "input",
            {
              type: "number",
              value: newTunnel.port,
              onChange: (e) => setNewTunnel({ ...newTunnel, port: e.target.value }),
              placeholder: "8080",
              className: "w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500", children: "The port your mock server is running on locally" })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: () => setIsCreateModalOpen(false),
            className: "px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors",
            children: "Cancel"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: handleCreate,
            disabled: !newTunnel.name || !newTunnel.port,
            className: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
            children: "Start Tunnel"
          }
        )
      ] })
    ] }) })
  ] });
};
const FederationPage = () => {
  const [workspaces, setWorkspaces] = reactExports.useState([
    {
      id: "ws_abc123",
      name: "Payment Team",
      url: "https://mockforge.payment-team.internal",
      status: "connected",
      shared_contracts: 12,
      last_sync: (/* @__PURE__ */ new Date()).toISOString()
    },
    {
      id: "ws_xyz789",
      name: "Inventory Service",
      url: "https://mockforge.inventory.internal",
      status: "disconnected",
      shared_contracts: 5,
      last_sync: new Date(Date.now() - 864e5).toISOString()
    }
  ]);
  const [isConnectModalOpen, setIsConnectModalOpen] = reactExports.useState(false);
  const [newConnection, setNewConnection] = reactExports.useState({ url: "", token: "" });
  const handleConnect = () => {
    const workspace = {
      id: `ws_${Math.random().toString(36).substr(2, 9)}`,
      name: "New Workspace",
      // In real app, would fetch name from URL
      url: newConnection.url,
      status: "connected",
      shared_contracts: 0,
      last_sync: (/* @__PURE__ */ new Date()).toISOString()
    };
    setWorkspaces([...workspaces, workspace]);
    setIsConnectModalOpen(false);
    setNewConnection({ url: "", token: "" });
  };
  const handleDisconnect = (id) => {
    setWorkspaces(workspaces.map(
      (ws) => ws.id === id ? { ...ws, status: "disconnected" } : ws
    ));
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 max-w-7xl mx-auto", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-start mb-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2", children: "Federation" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-gray-600 dark:text-gray-400", children: "Connect and compose multiple MockForge workspaces into a unified virtual system." })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setIsConnectModalOpen(true),
          className: "flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "w-4 h-4 mr-2" }),
            "Connect Workspace"
          ]
        }
      )
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-3 gap-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "md:col-span-1", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4", children: "Federation Status" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-4 mb-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-3 bg-blue-100 dark:bg-blue-900/30 rounded-full text-blue-600 dark:text-blue-400", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Share2, { className: "w-8 h-8" }) }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-3xl font-bold text-gray-900 dark:text-gray-100", children: workspaces.filter((w) => w.status === "connected").length }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-gray-500 dark:text-gray-400", children: "Active Connections" })
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("hr", { className: "border-gray-200 dark:border-gray-700 my-4" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 leading-relaxed", children: "Federation allows you to import contracts, fixtures, and scenarios from other workspaces. Changes in upstream workspaces can trigger alerts or automated updates." })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "md:col-span-2", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-6 border-b border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: "Connected Workspaces" }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("ul", { className: "divide-y divide-gray-200 dark:divide-gray-700", children: workspaces.map((ws) => /* @__PURE__ */ jsxRuntimeExports.jsx("li", { className: "p-6 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-600 dark:text-gray-300", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Building2, { className: "w-6 h-6" }) }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-medium text-gray-900 dark:text-gray-100", children: ws.name }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: `inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border ${ws.status === "connected" ? "bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30" : "bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700"}`, children: ws.status })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 mt-1 text-sm text-gray-500 dark:text-gray-400", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-mono", children: ws.url }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "•" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
                  ws.shared_contracts,
                  " shared contracts"
                ] })
              ] })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("button", { className: "p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "w-5 h-5" }) }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "button",
              {
                onClick: () => handleDisconnect(ws.id),
                className: `p-2 rounded-lg transition-colors ${ws.status === "connected" ? "text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20" : "text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"}`,
                title: ws.status === "connected" ? "Disconnect" : "Connect",
                children: ws.status === "connected" ? /* @__PURE__ */ jsxRuntimeExports.jsx(Link2Off, { className: "w-5 h-5" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Link, { className: "w-5 h-5" })
              }
            )
          ] })
        ] }) }, ws.id)) })
      ] }) })
    ] }),
    isConnectModalOpen && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-6 border-b border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-xl font-semibold text-gray-900 dark:text-gray-100", children: "Connect Remote Workspace" }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 p-4 rounded-lg text-sm flex gap-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CircleAlert, { className: "w-5 h-5 shrink-0" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { children: "Enter the URL and access token of the MockForge workspace you want to connect to." })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300", children: "Workspace URL" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "input",
            {
              type: "text",
              value: newConnection.url,
              onChange: (e) => setNewConnection({ ...newConnection, url: e.target.value }),
              placeholder: "https://mockforge.example.com",
              className: "w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300", children: "Access Token" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "input",
            {
              type: "password",
              value: newConnection.token,
              onChange: (e) => setNewConnection({ ...newConnection, token: e.target.value }),
              placeholder: "mf_token_...",
              className: "w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
            }
          )
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: () => setIsConnectModalOpen(false),
            className: "px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors",
            children: "Cancel"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "button",
          {
            onClick: handleConnect,
            disabled: !newConnection.url || !newConnection.token,
            className: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
            children: "Connect"
          }
        )
      ] })
    ] }) })
  ] });
};
var jt = (n) => {
  switch (n) {
    case "success":
      return ee;
    case "info":
      return ae;
    case "warning":
      return oe;
    case "error":
      return se;
    default:
      return null;
  }
}, te = Array(12).fill(0), Yt = ({ visible: n, className: e }) => React.createElement("div", { className: ["sonner-loading-wrapper", e].filter(Boolean).join(" "), "data-visible": n }, React.createElement("div", { className: "sonner-spinner" }, te.map((t, a) => React.createElement("div", { className: "sonner-loading-bar", key: `spinner-bar-${a}` })))), ee = React.createElement("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 20 20", fill: "currentColor", height: "20", width: "20" }, React.createElement("path", { fillRule: "evenodd", d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.857-9.809a.75.75 0 00-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 10-1.06 1.061l2.5 2.5a.75.75 0 001.137-.089l4-5.5z", clipRule: "evenodd" })), oe = React.createElement("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 24 24", fill: "currentColor", height: "20", width: "20" }, React.createElement("path", { fillRule: "evenodd", d: "M9.401 3.003c1.155-2 4.043-2 5.197 0l7.355 12.748c1.154 2-.29 4.5-2.599 4.5H4.645c-2.309 0-3.752-2.5-2.598-4.5L9.4 3.003zM12 8.25a.75.75 0 01.75.75v3.75a.75.75 0 01-1.5 0V9a.75.75 0 01.75-.75zm0 8.25a.75.75 0 100-1.5.75.75 0 000 1.5z", clipRule: "evenodd" })), ae = React.createElement("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 20 20", fill: "currentColor", height: "20", width: "20" }, React.createElement("path", { fillRule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a.75.75 0 000 1.5h.253a.25.25 0 01.244.304l-.459 2.066A1.75 1.75 0 0010.747 15H11a.75.75 0 000-1.5h-.253a.25.25 0 01-.244-.304l.459-2.066A1.75 1.75 0 009.253 9H9z", clipRule: "evenodd" })), se = React.createElement("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 20 20", fill: "currentColor", height: "20", width: "20" }, React.createElement("path", { fillRule: "evenodd", d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-5a.75.75 0 01.75.75v4.5a.75.75 0 01-1.5 0v-4.5A.75.75 0 0110 5zm0 10a1 1 0 100-2 1 1 0 000 2z", clipRule: "evenodd" })), Ot = React.createElement("svg", { xmlns: "http://www.w3.org/2000/svg", width: "12", height: "12", viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: "1.5", strokeLinecap: "round", strokeLinejoin: "round" }, React.createElement("line", { x1: "18", y1: "6", x2: "6", y2: "18" }), React.createElement("line", { x1: "6", y1: "6", x2: "18", y2: "18" }));
var Ft = () => {
  let [n, e] = React.useState(document.hidden);
  return React.useEffect(() => {
    let t = () => {
      e(document.hidden);
    };
    return document.addEventListener("visibilitychange", t), () => window.removeEventListener("visibilitychange", t);
  }, []), n;
};
var bt = 1, yt = class {
  constructor() {
    this.subscribe = (e) => (this.subscribers.push(e), () => {
      let t = this.subscribers.indexOf(e);
      this.subscribers.splice(t, 1);
    });
    this.publish = (e) => {
      this.subscribers.forEach((t) => t(e));
    };
    this.addToast = (e) => {
      this.publish(e), this.toasts = [...this.toasts, e];
    };
    this.create = (e) => {
      var S;
      let { message: t, ...a } = e, u = typeof (e == null ? void 0 : e.id) == "number" || ((S = e.id) == null ? void 0 : S.length) > 0 ? e.id : bt++, f = this.toasts.find((g) => g.id === u), w = e.dismissible === void 0 ? true : e.dismissible;
      return this.dismissedToasts.has(u) && this.dismissedToasts.delete(u), f ? this.toasts = this.toasts.map((g) => g.id === u ? (this.publish({ ...g, ...e, id: u, title: t }), { ...g, ...e, id: u, dismissible: w, title: t }) : g) : this.addToast({ title: t, ...a, dismissible: w, id: u }), u;
    };
    this.dismiss = (e) => (this.dismissedToasts.add(e), e || this.toasts.forEach((t) => {
      this.subscribers.forEach((a) => a({ id: t.id, dismiss: true }));
    }), this.subscribers.forEach((t) => t({ id: e, dismiss: true })), e);
    this.message = (e, t) => this.create({ ...t, message: e });
    this.error = (e, t) => this.create({ ...t, message: e, type: "error" });
    this.success = (e, t) => this.create({ ...t, type: "success", message: e });
    this.info = (e, t) => this.create({ ...t, type: "info", message: e });
    this.warning = (e, t) => this.create({ ...t, type: "warning", message: e });
    this.loading = (e, t) => this.create({ ...t, type: "loading", message: e });
    this.promise = (e, t) => {
      if (!t) return;
      let a;
      t.loading !== void 0 && (a = this.create({ ...t, promise: e, type: "loading", message: t.loading, description: typeof t.description != "function" ? t.description : void 0 }));
      let u = e instanceof Promise ? e : e(), f = a !== void 0, w, S = u.then(async (i) => {
        if (w = ["resolve", i], React.isValidElement(i)) f = false, this.create({ id: a, type: "default", message: i });
        else if (ie(i) && !i.ok) {
          f = false;
          let T = typeof t.error == "function" ? await t.error(`HTTP error! status: ${i.status}`) : t.error, F = typeof t.description == "function" ? await t.description(`HTTP error! status: ${i.status}`) : t.description;
          this.create({ id: a, type: "error", message: T, description: F });
        } else if (t.success !== void 0) {
          f = false;
          let T = typeof t.success == "function" ? await t.success(i) : t.success, F = typeof t.description == "function" ? await t.description(i) : t.description;
          this.create({ id: a, type: "success", message: T, description: F });
        }
      }).catch(async (i) => {
        if (w = ["reject", i], t.error !== void 0) {
          f = false;
          let D = typeof t.error == "function" ? await t.error(i) : t.error, T = typeof t.description == "function" ? await t.description(i) : t.description;
          this.create({ id: a, type: "error", message: D, description: T });
        }
      }).finally(() => {
        var i;
        f && (this.dismiss(a), a = void 0), (i = t.finally) == null || i.call(t);
      }), g = () => new Promise((i, D) => S.then(() => w[0] === "reject" ? D(w[1]) : i(w[1])).catch(D));
      return typeof a != "string" && typeof a != "number" ? { unwrap: g } : Object.assign(a, { unwrap: g });
    };
    this.custom = (e, t) => {
      let a = (t == null ? void 0 : t.id) || bt++;
      return this.create({ jsx: e(a), id: a, ...t }), a;
    };
    this.getActiveToasts = () => this.toasts.filter((e) => !this.dismissedToasts.has(e.id));
    this.subscribers = [], this.toasts = [], this.dismissedToasts = /* @__PURE__ */ new Set();
  }
}, v = new yt(), ne = (n, e) => {
  let t = (e == null ? void 0 : e.id) || bt++;
  return v.addToast({ title: n, ...e, id: t }), t;
}, ie = (n) => n && typeof n == "object" && "ok" in n && typeof n.ok == "boolean" && "status" in n && typeof n.status == "number", le = ne, ce = () => v.toasts, de = () => v.getActiveToasts(), ue = Object.assign(le, { success: v.success, info: v.info, warning: v.warning, error: v.error, custom: v.custom, message: v.message, promise: v.promise, dismiss: v.dismiss, loading: v.loading }, { getHistory: ce, getToasts: de });
function wt(n, { insertAt: e } = {}) {
  if (typeof document == "undefined") return;
  let t = document.head || document.getElementsByTagName("head")[0], a = document.createElement("style");
  a.type = "text/css", e === "top" && t.firstChild ? t.insertBefore(a, t.firstChild) : t.appendChild(a), a.styleSheet ? a.styleSheet.cssText = n : a.appendChild(document.createTextNode(n));
}
wt(`:where(html[dir="ltr"]),:where([data-sonner-toaster][dir="ltr"]){--toast-icon-margin-start: -3px;--toast-icon-margin-end: 4px;--toast-svg-margin-start: -1px;--toast-svg-margin-end: 0px;--toast-button-margin-start: auto;--toast-button-margin-end: 0;--toast-close-button-start: 0;--toast-close-button-end: unset;--toast-close-button-transform: translate(-35%, -35%)}:where(html[dir="rtl"]),:where([data-sonner-toaster][dir="rtl"]){--toast-icon-margin-start: 4px;--toast-icon-margin-end: -3px;--toast-svg-margin-start: 0px;--toast-svg-margin-end: -1px;--toast-button-margin-start: 0;--toast-button-margin-end: auto;--toast-close-button-start: unset;--toast-close-button-end: 0;--toast-close-button-transform: translate(35%, -35%)}:where([data-sonner-toaster]){position:fixed;width:var(--width);font-family:ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica Neue,Arial,Noto Sans,sans-serif,Apple Color Emoji,Segoe UI Emoji,Segoe UI Symbol,Noto Color Emoji;--gray1: hsl(0, 0%, 99%);--gray2: hsl(0, 0%, 97.3%);--gray3: hsl(0, 0%, 95.1%);--gray4: hsl(0, 0%, 93%);--gray5: hsl(0, 0%, 90.9%);--gray6: hsl(0, 0%, 88.7%);--gray7: hsl(0, 0%, 85.8%);--gray8: hsl(0, 0%, 78%);--gray9: hsl(0, 0%, 56.1%);--gray10: hsl(0, 0%, 52.3%);--gray11: hsl(0, 0%, 43.5%);--gray12: hsl(0, 0%, 9%);--border-radius: 8px;box-sizing:border-box;padding:0;margin:0;list-style:none;outline:none;z-index:999999999;transition:transform .4s ease}:where([data-sonner-toaster][data-lifted="true"]){transform:translateY(-10px)}@media (hover: none) and (pointer: coarse){:where([data-sonner-toaster][data-lifted="true"]){transform:none}}:where([data-sonner-toaster][data-x-position="right"]){right:var(--offset-right)}:where([data-sonner-toaster][data-x-position="left"]){left:var(--offset-left)}:where([data-sonner-toaster][data-x-position="center"]){left:50%;transform:translate(-50%)}:where([data-sonner-toaster][data-y-position="top"]){top:var(--offset-top)}:where([data-sonner-toaster][data-y-position="bottom"]){bottom:var(--offset-bottom)}:where([data-sonner-toast]){--y: translateY(100%);--lift-amount: calc(var(--lift) * var(--gap));z-index:var(--z-index);position:absolute;opacity:0;transform:var(--y);filter:blur(0);touch-action:none;transition:transform .4s,opacity .4s,height .4s,box-shadow .2s;box-sizing:border-box;outline:none;overflow-wrap:anywhere}:where([data-sonner-toast][data-styled="true"]){padding:16px;background:var(--normal-bg);border:1px solid var(--normal-border);color:var(--normal-text);border-radius:var(--border-radius);box-shadow:0 4px 12px #0000001a;width:var(--width);font-size:13px;display:flex;align-items:center;gap:6px}:where([data-sonner-toast]:focus-visible){box-shadow:0 4px 12px #0000001a,0 0 0 2px #0003}:where([data-sonner-toast][data-y-position="top"]){top:0;--y: translateY(-100%);--lift: 1;--lift-amount: calc(1 * var(--gap))}:where([data-sonner-toast][data-y-position="bottom"]){bottom:0;--y: translateY(100%);--lift: -1;--lift-amount: calc(var(--lift) * var(--gap))}:where([data-sonner-toast]) :where([data-description]){font-weight:400;line-height:1.4;color:inherit}:where([data-sonner-toast]) :where([data-title]){font-weight:500;line-height:1.5;color:inherit}:where([data-sonner-toast]) :where([data-icon]){display:flex;height:16px;width:16px;position:relative;justify-content:flex-start;align-items:center;flex-shrink:0;margin-left:var(--toast-icon-margin-start);margin-right:var(--toast-icon-margin-end)}:where([data-sonner-toast][data-promise="true"]) :where([data-icon])>svg{opacity:0;transform:scale(.8);transform-origin:center;animation:sonner-fade-in .3s ease forwards}:where([data-sonner-toast]) :where([data-icon])>*{flex-shrink:0}:where([data-sonner-toast]) :where([data-icon]) svg{margin-left:var(--toast-svg-margin-start);margin-right:var(--toast-svg-margin-end)}:where([data-sonner-toast]) :where([data-content]){display:flex;flex-direction:column;gap:2px}[data-sonner-toast][data-styled=true] [data-button]{border-radius:4px;padding-left:8px;padding-right:8px;height:24px;font-size:12px;color:var(--normal-bg);background:var(--normal-text);margin-left:var(--toast-button-margin-start);margin-right:var(--toast-button-margin-end);border:none;cursor:pointer;outline:none;display:flex;align-items:center;flex-shrink:0;transition:opacity .4s,box-shadow .2s}:where([data-sonner-toast]) :where([data-button]):focus-visible{box-shadow:0 0 0 2px #0006}:where([data-sonner-toast]) :where([data-button]):first-of-type{margin-left:var(--toast-button-margin-start);margin-right:var(--toast-button-margin-end)}:where([data-sonner-toast]) :where([data-cancel]){color:var(--normal-text);background:rgba(0,0,0,.08)}:where([data-sonner-toast][data-theme="dark"]) :where([data-cancel]){background:rgba(255,255,255,.3)}:where([data-sonner-toast]) :where([data-close-button]){position:absolute;left:var(--toast-close-button-start);right:var(--toast-close-button-end);top:0;height:20px;width:20px;display:flex;justify-content:center;align-items:center;padding:0;color:var(--gray12);border:1px solid var(--gray4);transform:var(--toast-close-button-transform);border-radius:50%;cursor:pointer;z-index:1;transition:opacity .1s,background .2s,border-color .2s}[data-sonner-toast] [data-close-button]{background:var(--gray1)}:where([data-sonner-toast]) :where([data-close-button]):focus-visible{box-shadow:0 4px 12px #0000001a,0 0 0 2px #0003}:where([data-sonner-toast]) :where([data-disabled="true"]){cursor:not-allowed}:where([data-sonner-toast]):hover :where([data-close-button]):hover{background:var(--gray2);border-color:var(--gray5)}:where([data-sonner-toast][data-swiping="true"]):before{content:"";position:absolute;left:-50%;right:-50%;height:100%;z-index:-1}:where([data-sonner-toast][data-y-position="top"][data-swiping="true"]):before{bottom:50%;transform:scaleY(3) translateY(50%)}:where([data-sonner-toast][data-y-position="bottom"][data-swiping="true"]):before{top:50%;transform:scaleY(3) translateY(-50%)}:where([data-sonner-toast][data-swiping="false"][data-removed="true"]):before{content:"";position:absolute;inset:0;transform:scaleY(2)}:where([data-sonner-toast]):after{content:"";position:absolute;left:0;height:calc(var(--gap) + 1px);bottom:100%;width:100%}:where([data-sonner-toast][data-mounted="true"]){--y: translateY(0);opacity:1}:where([data-sonner-toast][data-expanded="false"][data-front="false"]){--scale: var(--toasts-before) * .05 + 1;--y: translateY(calc(var(--lift-amount) * var(--toasts-before))) scale(calc(-1 * var(--scale)));height:var(--front-toast-height)}:where([data-sonner-toast])>*{transition:opacity .4s}:where([data-sonner-toast][data-expanded="false"][data-front="false"][data-styled="true"])>*{opacity:0}:where([data-sonner-toast][data-visible="false"]){opacity:0;pointer-events:none}:where([data-sonner-toast][data-mounted="true"][data-expanded="true"]){--y: translateY(calc(var(--lift) * var(--offset)));height:var(--initial-height)}:where([data-sonner-toast][data-removed="true"][data-front="true"][data-swipe-out="false"]){--y: translateY(calc(var(--lift) * -100%));opacity:0}:where([data-sonner-toast][data-removed="true"][data-front="false"][data-swipe-out="false"][data-expanded="true"]){--y: translateY(calc(var(--lift) * var(--offset) + var(--lift) * -100%));opacity:0}:where([data-sonner-toast][data-removed="true"][data-front="false"][data-swipe-out="false"][data-expanded="false"]){--y: translateY(40%);opacity:0;transition:transform .5s,opacity .2s}:where([data-sonner-toast][data-removed="true"][data-front="false"]):before{height:calc(var(--initial-height) + 20%)}[data-sonner-toast][data-swiping=true]{transform:var(--y) translateY(var(--swipe-amount-y, 0px)) translate(var(--swipe-amount-x, 0px));transition:none}[data-sonner-toast][data-swiped=true]{user-select:none}[data-sonner-toast][data-swipe-out=true][data-y-position=bottom],[data-sonner-toast][data-swipe-out=true][data-y-position=top]{animation-duration:.2s;animation-timing-function:ease-out;animation-fill-mode:forwards}[data-sonner-toast][data-swipe-out=true][data-swipe-direction=left]{animation-name:swipe-out-left}[data-sonner-toast][data-swipe-out=true][data-swipe-direction=right]{animation-name:swipe-out-right}[data-sonner-toast][data-swipe-out=true][data-swipe-direction=up]{animation-name:swipe-out-up}[data-sonner-toast][data-swipe-out=true][data-swipe-direction=down]{animation-name:swipe-out-down}@keyframes swipe-out-left{0%{transform:var(--y) translate(var(--swipe-amount-x));opacity:1}to{transform:var(--y) translate(calc(var(--swipe-amount-x) - 100%));opacity:0}}@keyframes swipe-out-right{0%{transform:var(--y) translate(var(--swipe-amount-x));opacity:1}to{transform:var(--y) translate(calc(var(--swipe-amount-x) + 100%));opacity:0}}@keyframes swipe-out-up{0%{transform:var(--y) translateY(var(--swipe-amount-y));opacity:1}to{transform:var(--y) translateY(calc(var(--swipe-amount-y) - 100%));opacity:0}}@keyframes swipe-out-down{0%{transform:var(--y) translateY(var(--swipe-amount-y));opacity:1}to{transform:var(--y) translateY(calc(var(--swipe-amount-y) + 100%));opacity:0}}@media (max-width: 600px){[data-sonner-toaster]{position:fixed;right:var(--mobile-offset-right);left:var(--mobile-offset-left);width:100%}[data-sonner-toaster][dir=rtl]{left:calc(var(--mobile-offset-left) * -1)}[data-sonner-toaster] [data-sonner-toast]{left:0;right:0;width:calc(100% - var(--mobile-offset-left) * 2)}[data-sonner-toaster][data-x-position=left]{left:var(--mobile-offset-left)}[data-sonner-toaster][data-y-position=bottom]{bottom:var(--mobile-offset-bottom)}[data-sonner-toaster][data-y-position=top]{top:var(--mobile-offset-top)}[data-sonner-toaster][data-x-position=center]{left:var(--mobile-offset-left);right:var(--mobile-offset-right);transform:none}}[data-sonner-toaster][data-theme=light]{--normal-bg: #fff;--normal-border: var(--gray4);--normal-text: var(--gray12);--success-bg: hsl(143, 85%, 96%);--success-border: hsl(145, 92%, 91%);--success-text: hsl(140, 100%, 27%);--info-bg: hsl(208, 100%, 97%);--info-border: hsl(221, 91%, 91%);--info-text: hsl(210, 92%, 45%);--warning-bg: hsl(49, 100%, 97%);--warning-border: hsl(49, 91%, 91%);--warning-text: hsl(31, 92%, 45%);--error-bg: hsl(359, 100%, 97%);--error-border: hsl(359, 100%, 94%);--error-text: hsl(360, 100%, 45%)}[data-sonner-toaster][data-theme=light] [data-sonner-toast][data-invert=true]{--normal-bg: #000;--normal-border: hsl(0, 0%, 20%);--normal-text: var(--gray1)}[data-sonner-toaster][data-theme=dark] [data-sonner-toast][data-invert=true]{--normal-bg: #fff;--normal-border: var(--gray3);--normal-text: var(--gray12)}[data-sonner-toaster][data-theme=dark]{--normal-bg: #000;--normal-bg-hover: hsl(0, 0%, 12%);--normal-border: hsl(0, 0%, 20%);--normal-border-hover: hsl(0, 0%, 25%);--normal-text: var(--gray1);--success-bg: hsl(150, 100%, 6%);--success-border: hsl(147, 100%, 12%);--success-text: hsl(150, 86%, 65%);--info-bg: hsl(215, 100%, 6%);--info-border: hsl(223, 100%, 12%);--info-text: hsl(216, 87%, 65%);--warning-bg: hsl(64, 100%, 6%);--warning-border: hsl(60, 100%, 12%);--warning-text: hsl(46, 87%, 65%);--error-bg: hsl(358, 76%, 10%);--error-border: hsl(357, 89%, 16%);--error-text: hsl(358, 100%, 81%)}[data-sonner-toaster][data-theme=dark] [data-sonner-toast] [data-close-button]{background:var(--normal-bg);border-color:var(--normal-border);color:var(--normal-text)}[data-sonner-toaster][data-theme=dark] [data-sonner-toast] [data-close-button]:hover{background:var(--normal-bg-hover);border-color:var(--normal-border-hover)}[data-rich-colors=true][data-sonner-toast][data-type=success],[data-rich-colors=true][data-sonner-toast][data-type=success] [data-close-button]{background:var(--success-bg);border-color:var(--success-border);color:var(--success-text)}[data-rich-colors=true][data-sonner-toast][data-type=info],[data-rich-colors=true][data-sonner-toast][data-type=info] [data-close-button]{background:var(--info-bg);border-color:var(--info-border);color:var(--info-text)}[data-rich-colors=true][data-sonner-toast][data-type=warning],[data-rich-colors=true][data-sonner-toast][data-type=warning] [data-close-button]{background:var(--warning-bg);border-color:var(--warning-border);color:var(--warning-text)}[data-rich-colors=true][data-sonner-toast][data-type=error],[data-rich-colors=true][data-sonner-toast][data-type=error] [data-close-button]{background:var(--error-bg);border-color:var(--error-border);color:var(--error-text)}.sonner-loading-wrapper{--size: 16px;height:var(--size);width:var(--size);position:absolute;inset:0;z-index:10}.sonner-loading-wrapper[data-visible=false]{transform-origin:center;animation:sonner-fade-out .2s ease forwards}.sonner-spinner{position:relative;top:50%;left:50%;height:var(--size);width:var(--size)}.sonner-loading-bar{animation:sonner-spin 1.2s linear infinite;background:var(--gray11);border-radius:6px;height:8%;left:-10%;position:absolute;top:-3.9%;width:24%}.sonner-loading-bar:nth-child(1){animation-delay:-1.2s;transform:rotate(.0001deg) translate(146%)}.sonner-loading-bar:nth-child(2){animation-delay:-1.1s;transform:rotate(30deg) translate(146%)}.sonner-loading-bar:nth-child(3){animation-delay:-1s;transform:rotate(60deg) translate(146%)}.sonner-loading-bar:nth-child(4){animation-delay:-.9s;transform:rotate(90deg) translate(146%)}.sonner-loading-bar:nth-child(5){animation-delay:-.8s;transform:rotate(120deg) translate(146%)}.sonner-loading-bar:nth-child(6){animation-delay:-.7s;transform:rotate(150deg) translate(146%)}.sonner-loading-bar:nth-child(7){animation-delay:-.6s;transform:rotate(180deg) translate(146%)}.sonner-loading-bar:nth-child(8){animation-delay:-.5s;transform:rotate(210deg) translate(146%)}.sonner-loading-bar:nth-child(9){animation-delay:-.4s;transform:rotate(240deg) translate(146%)}.sonner-loading-bar:nth-child(10){animation-delay:-.3s;transform:rotate(270deg) translate(146%)}.sonner-loading-bar:nth-child(11){animation-delay:-.2s;transform:rotate(300deg) translate(146%)}.sonner-loading-bar:nth-child(12){animation-delay:-.1s;transform:rotate(330deg) translate(146%)}@keyframes sonner-fade-in{0%{opacity:0;transform:scale(.8)}to{opacity:1;transform:scale(1)}}@keyframes sonner-fade-out{0%{opacity:1;transform:scale(1)}to{opacity:0;transform:scale(.8)}}@keyframes sonner-spin{0%{opacity:1}to{opacity:.15}}@media (prefers-reduced-motion){[data-sonner-toast],[data-sonner-toast]>*,.sonner-loading-bar{transition:none!important;animation:none!important}}.sonner-loader{position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);transform-origin:center;transition:opacity .2s,transform .2s}.sonner-loader[data-visible=false]{opacity:0;transform:scale(.8) translate(-50%,-50%)}
`);
function tt(n) {
  return n.label !== void 0;
}
var pe = 3, me = "32px", ge = "16px", Wt = 4e3, he = 356, be = 14, ye = 20, we = 200;
function M(...n) {
  return n.filter(Boolean).join(" ");
}
function xe(n) {
  let [e, t] = n.split("-"), a = [];
  return e && a.push(e), t && a.push(t), a;
}
var ve = (n) => {
  var Dt, Pt, Nt, Bt, Ct, kt, It, Mt, Ht, At, Lt;
  let { invert: e, toast: t, unstyled: a, interacting: u, setHeights: f, visibleToasts: w, heights: S, index: g, toasts: i, expanded: D, removeToast: T, defaultRichColors: F, closeButton: et, style: ut, cancelButtonStyle: ft, actionButtonStyle: l, className: ot = "", descriptionClassName: at = "", duration: X2, position: st, gap: pt, loadingIcon: rt, expandByDefault: B, classNames: s, icons: P, closeButtonAriaLabel: nt = "Close toast", pauseWhenPageIsHidden: it } = n, [Y, C] = React.useState(null), [lt, J] = React.useState(null), [W, H] = React.useState(false), [A, mt] = React.useState(false), [L, z] = React.useState(false), [ct, d] = React.useState(false), [h, y] = React.useState(false), [R, j] = React.useState(0), [p, _] = React.useState(0), O = React.useRef(t.duration || X2 || Wt), G = React.useRef(null), k = React.useRef(null), Vt = g === 0, Ut = g + 1 <= w, N = t.type, V = t.dismissible !== false, Kt = t.className || "", Xt = t.descriptionClassName || "", dt = React.useMemo(() => S.findIndex((r) => r.toastId === t.id) || 0, [S, t.id]), Jt = React.useMemo(() => {
    var r;
    return (r = t.closeButton) != null ? r : et;
  }, [t.closeButton, et]), Tt = React.useMemo(() => t.duration || X2 || Wt, [t.duration, X2]), gt = React.useRef(0), U = React.useRef(0), St = React.useRef(0), K = React.useRef(null), [Gt, Qt] = st.split("-"), Rt = React.useMemo(() => S.reduce((r, m, c) => c >= dt ? r : r + m.height, 0), [S, dt]), Et = Ft(), qt = t.invert || e, ht = N === "loading";
  U.current = React.useMemo(() => dt * pt + Rt, [dt, Rt]), React.useEffect(() => {
    O.current = Tt;
  }, [Tt]), React.useEffect(() => {
    H(true);
  }, []), React.useEffect(() => {
    let r = k.current;
    if (r) {
      let m = r.getBoundingClientRect().height;
      return _(m), f((c) => [{ toastId: t.id, height: m, position: t.position }, ...c]), () => f((c) => c.filter((b) => b.toastId !== t.id));
    }
  }, [f, t.id]), React.useLayoutEffect(() => {
    if (!W) return;
    let r = k.current, m = r.style.height;
    r.style.height = "auto";
    let c = r.getBoundingClientRect().height;
    r.style.height = m, _(c), f((b) => b.find((x) => x.toastId === t.id) ? b.map((x) => x.toastId === t.id ? { ...x, height: c } : x) : [{ toastId: t.id, height: c, position: t.position }, ...b]);
  }, [W, t.title, t.description, f, t.id]);
  let $ = React.useCallback(() => {
    mt(true), j(U.current), f((r) => r.filter((m) => m.toastId !== t.id)), setTimeout(() => {
      T(t);
    }, we);
  }, [t, T, f, U]);
  React.useEffect(() => {
    if (t.promise && N === "loading" || t.duration === 1 / 0 || t.type === "loading") return;
    let r;
    return D || u || it && Et ? (() => {
      if (St.current < gt.current) {
        let b = (/* @__PURE__ */ new Date()).getTime() - gt.current;
        O.current = O.current - b;
      }
      St.current = (/* @__PURE__ */ new Date()).getTime();
    })() : (() => {
      O.current !== 1 / 0 && (gt.current = (/* @__PURE__ */ new Date()).getTime(), r = setTimeout(() => {
        var b;
        (b = t.onAutoClose) == null || b.call(t, t), $();
      }, O.current));
    })(), () => clearTimeout(r);
  }, [D, u, t, N, it, Et, $]), React.useEffect(() => {
    t.delete && $();
  }, [$, t.delete]);
  function Zt() {
    var r, m, c;
    return P != null && P.loading ? React.createElement("div", { className: M(s == null ? void 0 : s.loader, (r = t == null ? void 0 : t.classNames) == null ? void 0 : r.loader, "sonner-loader"), "data-visible": N === "loading" }, P.loading) : rt ? React.createElement("div", { className: M(s == null ? void 0 : s.loader, (m = t == null ? void 0 : t.classNames) == null ? void 0 : m.loader, "sonner-loader"), "data-visible": N === "loading" }, rt) : React.createElement(Yt, { className: M(s == null ? void 0 : s.loader, (c = t == null ? void 0 : t.classNames) == null ? void 0 : c.loader), visible: N === "loading" });
  }
  return React.createElement("li", { tabIndex: 0, ref: k, className: M(ot, Kt, s == null ? void 0 : s.toast, (Dt = t == null ? void 0 : t.classNames) == null ? void 0 : Dt.toast, s == null ? void 0 : s.default, s == null ? void 0 : s[N], (Pt = t == null ? void 0 : t.classNames) == null ? void 0 : Pt[N]), "data-sonner-toast": "", "data-rich-colors": (Nt = t.richColors) != null ? Nt : F, "data-styled": !(t.jsx || t.unstyled || a), "data-mounted": W, "data-promise": !!t.promise, "data-swiped": h, "data-removed": A, "data-visible": Ut, "data-y-position": Gt, "data-x-position": Qt, "data-index": g, "data-front": Vt, "data-swiping": L, "data-dismissible": V, "data-type": N, "data-invert": qt, "data-swipe-out": ct, "data-swipe-direction": lt, "data-expanded": !!(D || B && W), style: { "--index": g, "--toasts-before": g, "--z-index": i.length - g, "--offset": `${A ? R : U.current}px`, "--initial-height": B ? "auto" : `${p}px`, ...ut, ...t.style }, onDragEnd: () => {
    z(false), C(null), K.current = null;
  }, onPointerDown: (r) => {
    ht || !V || (G.current = /* @__PURE__ */ new Date(), j(U.current), r.target.setPointerCapture(r.pointerId), r.target.tagName !== "BUTTON" && (z(true), K.current = { x: r.clientX, y: r.clientY }));
  }, onPointerUp: () => {
    var x, Q, q, Z;
    if (ct || !V) return;
    K.current = null;
    let r = Number(((x = k.current) == null ? void 0 : x.style.getPropertyValue("--swipe-amount-x").replace("px", "")) || 0), m = Number(((Q = k.current) == null ? void 0 : Q.style.getPropertyValue("--swipe-amount-y").replace("px", "")) || 0), c = (/* @__PURE__ */ new Date()).getTime() - ((q = G.current) == null ? void 0 : q.getTime()), b = Y === "x" ? r : m, I = Math.abs(b) / c;
    if (Math.abs(b) >= ye || I > 0.11) {
      j(U.current), (Z = t.onDismiss) == null || Z.call(t, t), J(Y === "x" ? r > 0 ? "right" : "left" : m > 0 ? "down" : "up"), $(), d(true), y(false);
      return;
    }
    z(false), C(null);
  }, onPointerMove: (r) => {
    var Q, q, Z, zt;
    if (!K.current || !V || ((Q = window.getSelection()) == null ? void 0 : Q.toString().length) > 0) return;
    let c = r.clientY - K.current.y, b = r.clientX - K.current.x, I = (q = n.swipeDirections) != null ? q : xe(st);
    !Y && (Math.abs(b) > 1 || Math.abs(c) > 1) && C(Math.abs(b) > Math.abs(c) ? "x" : "y");
    let x = { x: 0, y: 0 };
    Y === "y" ? (I.includes("top") || I.includes("bottom")) && (I.includes("top") && c < 0 || I.includes("bottom") && c > 0) && (x.y = c) : Y === "x" && (I.includes("left") || I.includes("right")) && (I.includes("left") && b < 0 || I.includes("right") && b > 0) && (x.x = b), (Math.abs(x.x) > 0 || Math.abs(x.y) > 0) && y(true), (Z = k.current) == null || Z.style.setProperty("--swipe-amount-x", `${x.x}px`), (zt = k.current) == null || zt.style.setProperty("--swipe-amount-y", `${x.y}px`);
  } }, Jt && !t.jsx ? React.createElement("button", { "aria-label": nt, "data-disabled": ht, "data-close-button": true, onClick: ht || !V ? () => {
  } : () => {
    var r;
    $(), (r = t.onDismiss) == null || r.call(t, t);
  }, className: M(s == null ? void 0 : s.closeButton, (Bt = t == null ? void 0 : t.classNames) == null ? void 0 : Bt.closeButton) }, (Ct = P == null ? void 0 : P.close) != null ? Ct : Ot) : null, t.jsx || reactExports.isValidElement(t.title) ? t.jsx ? t.jsx : typeof t.title == "function" ? t.title() : t.title : React.createElement(React.Fragment, null, N || t.icon || t.promise ? React.createElement("div", { "data-icon": "", className: M(s == null ? void 0 : s.icon, (kt = t == null ? void 0 : t.classNames) == null ? void 0 : kt.icon) }, t.promise || t.type === "loading" && !t.icon ? t.icon || Zt() : null, t.type !== "loading" ? t.icon || (P == null ? void 0 : P[N]) || jt(N) : null) : null, React.createElement("div", { "data-content": "", className: M(s == null ? void 0 : s.content, (It = t == null ? void 0 : t.classNames) == null ? void 0 : It.content) }, React.createElement("div", { "data-title": "", className: M(s == null ? void 0 : s.title, (Mt = t == null ? void 0 : t.classNames) == null ? void 0 : Mt.title) }, typeof t.title == "function" ? t.title() : t.title), t.description ? React.createElement("div", { "data-description": "", className: M(at, Xt, s == null ? void 0 : s.description, (Ht = t == null ? void 0 : t.classNames) == null ? void 0 : Ht.description) }, typeof t.description == "function" ? t.description() : t.description) : null), reactExports.isValidElement(t.cancel) ? t.cancel : t.cancel && tt(t.cancel) ? React.createElement("button", { "data-button": true, "data-cancel": true, style: t.cancelButtonStyle || ft, onClick: (r) => {
    var m, c;
    tt(t.cancel) && V && ((c = (m = t.cancel).onClick) == null || c.call(m, r), $());
  }, className: M(s == null ? void 0 : s.cancelButton, (At = t == null ? void 0 : t.classNames) == null ? void 0 : At.cancelButton) }, t.cancel.label) : null, reactExports.isValidElement(t.action) ? t.action : t.action && tt(t.action) ? React.createElement("button", { "data-button": true, "data-action": true, style: t.actionButtonStyle || l, onClick: (r) => {
    var m, c;
    tt(t.action) && ((c = (m = t.action).onClick) == null || c.call(m, r), !r.defaultPrevented && $());
  }, className: M(s == null ? void 0 : s.actionButton, (Lt = t == null ? void 0 : t.classNames) == null ? void 0 : Lt.actionButton) }, t.action.label) : null));
};
function _t() {
  if (typeof window == "undefined" || typeof document == "undefined") return "ltr";
  let n = document.documentElement.getAttribute("dir");
  return n === "auto" || !n ? window.getComputedStyle(document.documentElement).direction : n;
}
function Te(n, e) {
  let t = {};
  return [n, e].forEach((a, u) => {
    let f = u === 1, w = f ? "--mobile-offset" : "--offset", S = f ? ge : me;
    function g(i) {
      ["top", "right", "bottom", "left"].forEach((D) => {
        t[`${w}-${D}`] = typeof i == "number" ? `${i}px` : i;
      });
    }
    typeof a == "number" || typeof a == "string" ? g(a) : typeof a == "object" ? ["top", "right", "bottom", "left"].forEach((i) => {
      a[i] === void 0 ? t[`${w}-${i}`] = S : t[`${w}-${i}`] = typeof a[i] == "number" ? `${a[i]}px` : a[i];
    }) : g(S);
  }), t;
}
reactExports.forwardRef(function(e, t) {
  let { invert: a, position: u = "bottom-right", hotkey: f = ["altKey", "KeyT"], expand: w, closeButton: S, className: g, offset: i, mobileOffset: D, theme: T = "light", richColors: F, duration: et, style: ut, visibleToasts: ft = pe, toastOptions: l, dir: ot = _t(), gap: at = be, loadingIcon: X2, icons: st, containerAriaLabel: pt = "Notifications", pauseWhenPageIsHidden: rt } = e, [B, s] = React.useState([]), P = React.useMemo(() => Array.from(new Set([u].concat(B.filter((d) => d.position).map((d) => d.position)))), [B, u]), [nt, it] = React.useState([]), [Y, C] = React.useState(false), [lt, J] = React.useState(false), [W, H] = React.useState(T !== "system" ? T : typeof window != "undefined" && window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"), A = React.useRef(null), mt = f.join("+").replace(/Key/g, "").replace(/Digit/g, ""), L = React.useRef(null), z = React.useRef(false), ct = React.useCallback((d) => {
    s((h) => {
      var y;
      return (y = h.find((R) => R.id === d.id)) != null && y.delete || v.dismiss(d.id), h.filter(({ id: R }) => R !== d.id);
    });
  }, []);
  return React.useEffect(() => v.subscribe((d) => {
    if (d.dismiss) {
      s((h) => h.map((y) => y.id === d.id ? { ...y, delete: true } : y));
      return;
    }
    setTimeout(() => {
      ReactDOM.flushSync(() => {
        s((h) => {
          let y = h.findIndex((R) => R.id === d.id);
          return y !== -1 ? [...h.slice(0, y), { ...h[y], ...d }, ...h.slice(y + 1)] : [d, ...h];
        });
      });
    });
  }), []), React.useEffect(() => {
    if (T !== "system") {
      H(T);
      return;
    }
    if (T === "system" && (window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches ? H("dark") : H("light")), typeof window == "undefined") return;
    let d = window.matchMedia("(prefers-color-scheme: dark)");
    try {
      d.addEventListener("change", ({ matches: h }) => {
        H(h ? "dark" : "light");
      });
    } catch (h) {
      d.addListener(({ matches: y }) => {
        try {
          H(y ? "dark" : "light");
        } catch (R) {
          console.error(R);
        }
      });
    }
  }, [T]), React.useEffect(() => {
    B.length <= 1 && C(false);
  }, [B]), React.useEffect(() => {
    let d = (h) => {
      var R, j;
      f.every((p) => h[p] || h.code === p) && (C(true), (R = A.current) == null || R.focus()), h.code === "Escape" && (document.activeElement === A.current || (j = A.current) != null && j.contains(document.activeElement)) && C(false);
    };
    return document.addEventListener("keydown", d), () => document.removeEventListener("keydown", d);
  }, [f]), React.useEffect(() => {
    if (A.current) return () => {
      L.current && (L.current.focus({ preventScroll: true }), L.current = null, z.current = false);
    };
  }, [A.current]), React.createElement("section", { ref: t, "aria-label": `${pt} ${mt}`, tabIndex: -1, "aria-live": "polite", "aria-relevant": "additions text", "aria-atomic": "false", suppressHydrationWarning: true }, P.map((d, h) => {
    var j;
    let [y, R] = d.split("-");
    return B.length ? React.createElement("ol", { key: d, dir: ot === "auto" ? _t() : ot, tabIndex: -1, ref: A, className: g, "data-sonner-toaster": true, "data-theme": W, "data-y-position": y, "data-lifted": Y && B.length > 1 && !w, "data-x-position": R, style: { "--front-toast-height": `${((j = nt[0]) == null ? void 0 : j.height) || 0}px`, "--width": `${he}px`, "--gap": `${at}px`, ...ut, ...Te(i, D) }, onBlur: (p) => {
      z.current && !p.currentTarget.contains(p.relatedTarget) && (z.current = false, L.current && (L.current.focus({ preventScroll: true }), L.current = null));
    }, onFocus: (p) => {
      p.target instanceof HTMLElement && p.target.dataset.dismissible === "false" || z.current || (z.current = true, L.current = p.relatedTarget);
    }, onMouseEnter: () => C(true), onMouseMove: () => C(true), onMouseLeave: () => {
      lt || C(false);
    }, onDragEnd: () => C(false), onPointerDown: (p) => {
      p.target instanceof HTMLElement && p.target.dataset.dismissible === "false" || J(true);
    }, onPointerUp: () => J(false) }, B.filter((p) => !p.position && h === 0 || p.position === d).map((p, _) => {
      var O, G;
      return React.createElement(ve, { key: p.id, icons: st, index: _, toast: p, defaultRichColors: F, duration: (O = l == null ? void 0 : l.duration) != null ? O : et, className: l == null ? void 0 : l.className, descriptionClassName: l == null ? void 0 : l.descriptionClassName, invert: a, visibleToasts: ft, closeButton: (G = l == null ? void 0 : l.closeButton) != null ? G : S, interacting: lt, position: d, style: l == null ? void 0 : l.style, unstyled: l == null ? void 0 : l.unstyled, classNames: l == null ? void 0 : l.classNames, cancelButtonStyle: l == null ? void 0 : l.cancelButtonStyle, actionButtonStyle: l == null ? void 0 : l.actionButtonStyle, removeToast: ct, toasts: B.filter((k) => k.position == p.position), heights: nt.filter((k) => k.position == p.position), setHeights: it, expandByDefault: w, gap: at, loadingIcon: X2, expanded: Y, pauseWhenPageIsHidden: rt, swipeDirections: e.swipeDirections });
    })) : null;
  }));
});
const SelectContext = reactExports.createContext(null);
const Select = ({ children, value, onValueChange, defaultValue, id, error, errorId }) => {
  const [internalValue, setInternalValue] = reactExports.useState(defaultValue || value || "");
  const [options, setOptions] = reactExports.useState([]);
  const currentValue = value || internalValue;
  const hasError = !!error;
  const handleValueChange = (newValue) => {
    setInternalValue(newValue);
    onValueChange == null ? void 0 : onValueChange(newValue);
  };
  const addOption = reactExports.useCallback((optValue, label) => {
    setOptions((prev) => {
      if (prev.some((o) => o.value === optValue)) return prev;
      return [...prev, { value: optValue, label }];
    });
  }, []);
  return /* @__PURE__ */ jsxRuntimeExports.jsx(SelectContext.Provider, { value: { value: currentValue, onValueChange: handleValueChange, id, options, addOption, hasError, errorId }, children });
};
const SelectValue = ({ placeholder, children }) => {
  const context = reactExports.useContext(SelectContext);
  return /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: children || (context == null ? void 0 : context.value) || placeholder });
};
const SelectTrigger = reactExports.forwardRef(({ className, children: _children, "aria-describedby": ariaDescribedby, ...props }, ref) => {
  const context = reactExports.useContext(SelectContext);
  const describedBy = [ariaDescribedby, context == null ? void 0 : context.errorId].filter(Boolean).join(" ") || void 0;
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "select",
    {
      ref,
      id: context == null ? void 0 : context.id,
      role: "combobox",
      value: context == null ? void 0 : context.value,
      onChange: (e) => context == null ? void 0 : context.onValueChange(e.target.value),
      "aria-invalid": (context == null ? void 0 : context.hasError) || void 0,
      "aria-describedby": describedBy,
      className: cn(
        "flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 [&>span]:line-clamp-1",
        (context == null ? void 0 : context.hasError) && "border-red-500 focus:ring-red-500",
        className
      ),
      ...props,
      children: context == null ? void 0 : context.options.map((opt) => /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: opt.value, children: opt.label }, opt.value))
    }
  );
});
SelectTrigger.displayName = "SelectTrigger";
const SelectContent = reactExports.forwardRef(({ className, children, position: _position = "popper", ...props }, ref) => /* @__PURE__ */ jsxRuntimeExports.jsx(
  "div",
  {
    ref,
    "data-select-content": true,
    className: cn(
      "relative z-50 max-h-96 min-w-[8rem] overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md p-1 hidden",
      className
    ),
    ...props,
    children
  }
));
SelectContent.displayName = "SelectContent";
const SelectLabel = reactExports.forwardRef(({ className, ...props }, ref) => /* @__PURE__ */ jsxRuntimeExports.jsx(
  "div",
  {
    ref,
    className: cn("py-1.5 pl-8 pr-2 text-sm font-semibold", className),
    ...props
  }
));
SelectLabel.displayName = "SelectLabel";
const SelectItem = reactExports.forwardRef(({ className, children, value, ...props }, ref) => {
  const context = reactExports.useContext(SelectContext);
  reactExports.useEffect(() => {
    if (context && typeof children === "string") {
      context.addOption(value, children);
    }
  }, [context, value, children]);
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "div",
    {
      ref,
      className: cn(
        "relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
        className
      ),
      "data-value": value,
      ...props,
      children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "absolute left-2 flex h-3.5 w-3.5 items-center justify-center", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Check, { className: "h-4 w-4" }) }),
        children
      ]
    }
  );
});
SelectItem.displayName = "SelectItem";
const SelectSeparator = reactExports.forwardRef(({ className, ...props }, ref) => /* @__PURE__ */ jsxRuntimeExports.jsx(
  "div",
  {
    ref,
    className: cn("-mx-1 my-1 h-px bg-muted", className),
    ...props
  }
));
SelectSeparator.displayName = "SelectSeparator";
function ContextMenuWithItems({ items, position, onClose, className }) {
  const menuRef = reactExports.useRef(null);
  reactExports.useEffect(() => {
    const handleClickOutside = (event) => {
      if (menuRef.current && !menuRef.current.contains(event.target)) {
        onClose();
      }
    };
    const handleEscape = (event) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);
  const adjustedPosition = React.useMemo(() => {
    if (!menuRef.current) return position;
    const menuRect = menuRef.current.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    let x = position.x;
    let y = position.y;
    if (x + menuRect.width > viewportWidth) {
      x = viewportWidth - menuRect.width - 10;
    }
    if (y + menuRect.height > viewportHeight) {
      y = viewportHeight - menuRect.height - 10;
    }
    return { x, y };
  }, [position]);
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      ref: menuRef,
      className: cn(
        "fixed z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-[200px]",
        className
      ),
      style: {
        left: adjustedPosition.x,
        top: adjustedPosition.y
      },
      children: items.map((item, index) => /* @__PURE__ */ jsxRuntimeExports.jsxs(React.Fragment, { children: [
        item.separator && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "border-t border-border my-1" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "button",
          {
            onClick: () => {
              if (!item.disabled) {
                item.onClick();
                onClose();
              }
            },
            disabled: item.disabled,
            className: cn(
              "w-full px-3 py-2 text-left text-sm hover:bg-accent hover:text-accent-foreground flex items-center gap-2",
              "focus:outline-none focus:bg-accent focus:text-accent-foreground",
              item.disabled && "opacity-50 cursor-not-allowed"
            ),
            children: [
              item.icon && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "w-4 h-4 flex items-center justify-center", children: item.icon }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: item.label })
            ]
          }
        )
      ] }, index))
    }
  );
}
function ContextMenu({ children }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(jsxRuntimeExports.Fragment, { children });
}
function ContextMenuTrigger({ children, onContextMenu }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { onContextMenu, children });
}
function ContextMenuContent({ children, className }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("bg-white border border-gray-200 rounded-md shadow-lg py-1 min-w-[200px]", className), children });
}
const PREDEFINED_COLORS = [
  { hex: "#3B82F6", name: "Blue" },
  { hex: "#EF4444", name: "Red" },
  { hex: "#10B981", name: "Green" },
  { hex: "#F59E0B", name: "Yellow" },
  { hex: "#8B5CF6", name: "Purple" },
  { hex: "#F97316", name: "Orange" },
  { hex: "#06B6D4", name: "Cyan" },
  { hex: "#84CC16", name: "Lime" }
];
function EnvironmentManager({ workspaceId, onEnvironmentSelect }) {
  const [isCreateDialogOpen, setIsCreateDialogOpen] = reactExports.useState(false);
  const [editingEnvironment, setEditingEnvironment] = reactExports.useState(null);
  const [createForm, setCreateForm] = reactExports.useState({ name: "", description: "" });
  const [editForm, setEditForm] = reactExports.useState({});
  const [selectedColor, setSelectedColor] = reactExports.useState(null);
  const [draggedItem, setDraggedItem] = reactExports.useState(null);
  const { data: environments, isLoading, error } = useEnvironments(workspaceId);
  const createEnvironment = useCreateEnvironment(workspaceId);
  const updateEnvironment = useUpdateEnvironment(workspaceId, (editingEnvironment == null ? void 0 : editingEnvironment.id) || "");
  const deleteEnvironment = useDeleteEnvironment(workspaceId);
  const setActiveEnvironment = useSetActiveEnvironment(workspaceId);
  const updateEnvironmentsOrder = useUpdateEnvironmentsOrder(workspaceId);
  const handleCreate = async () => {
    if (!createForm.name.trim()) {
      toast.error("Environment name is required");
      return;
    }
    try {
      await createEnvironment.mutateAsync({
        ...createForm,
        name: createForm.name.trim()
      });
      toast.success(`Environment "${createForm.name}" created successfully`);
      setCreateForm({ name: "", description: "" });
      setIsCreateDialogOpen(false);
    } catch {
      toast.error("Failed to create environment");
    }
  };
  const handleUpdate = async () => {
    if (!editingEnvironment) return;
    try {
      await updateEnvironment.mutateAsync(editForm);
      toast.success(`Environment "${editingEnvironment.name}" updated successfully`);
      setEditingEnvironment(null);
      setEditForm({});
      setSelectedColor(null);
    } catch {
      toast.error("Failed to update environment");
    }
  };
  const handleDelete = async (environment) => {
    if (environment.is_global) {
      toast.error("Cannot delete global environment");
      return;
    }
    if (!confirm(`Are you sure you want to delete "${environment.name}"? This action cannot be undone.`)) {
      return;
    }
    try {
      await deleteEnvironment.mutateAsync(environment.id);
      toast.success(`Environment "${environment.name}" deleted successfully`);
    } catch {
      toast.error("Failed to delete environment");
    }
  };
  const handleSetActive = async (environment) => {
    try {
      const envId = environment.is_global ? "global" : environment.id;
      await setActiveEnvironment.mutateAsync(envId);
      toast.success(`Switched to "${environment.name}" environment`);
      onEnvironmentSelect == null ? void 0 : onEnvironmentSelect(environment.id);
    } catch {
      toast.error("Failed to switch environment");
    }
  };
  const handleEdit = (environment) => {
    setEditingEnvironment(environment);
    setEditForm({
      name: environment.name,
      description: environment.description
    });
    setSelectedColor(environment.color || null);
  };
  const handleDragStart = (e, environmentId) => {
    setDraggedItem(environmentId);
    e.dataTransfer.effectAllowed = "move";
  };
  const handleDragOver = (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
  };
  const handleDrop = async (e, targetEnvironmentId) => {
    e.preventDefault();
    if (!draggedItem || draggedItem === targetEnvironmentId) {
      setDraggedItem(null);
      return;
    }
    if (!(environments == null ? void 0 : environments.environments)) {
      setDraggedItem(null);
      return;
    }
    try {
      const draggedIndex = environments.environments.findIndex((env) => env.id === draggedItem);
      const targetIndex = environments.environments.findIndex((env) => env.id === targetEnvironmentId);
      if (draggedIndex === -1 || targetIndex === -1) {
        setDraggedItem(null);
        return;
      }
      const newEnvironments = [...environments.environments];
      const [draggedEnv] = newEnvironments.splice(draggedIndex, 1);
      newEnvironments.splice(targetIndex, 0, draggedEnv);
      const environmentIds = newEnvironments.map((env) => env.id);
      try {
        await updateEnvironmentsOrder.mutateAsync(environmentIds);
        toast.success("Environment order updated");
      } catch {
        toast.error("Failed to update environment order");
        throw error;
      }
    } catch {
      toast.error("Failed to update environment order");
    } finally {
      setDraggedItem(null);
    }
  };
  const EnvironmentCard = ({ environment }) => {
    const { data: variables } = useEnvironmentVariables(workspaceId, environment.id);
    const isDragging = draggedItem === environment.id;
    return /* @__PURE__ */ jsxRuntimeExports.jsxs(ContextMenu, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(ContextMenuTrigger, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(
        ModernCard,
        {
          draggable: !environment.is_global,
          onDragStart: (e) => handleDragStart(e, environment.id),
          onDragOver: handleDragOver,
          onDrop: (e) => handleDrop(e, environment.id),
          className: `cursor-pointer transition-all duration-200 hover:shadow-lg ${environment.active ? "ring-2 ring-blue-500 bg-blue-50 dark:bg-blue-900/20" : "hover:bg-gray-50 dark:hover:bg-gray-800/50"} ${isDragging ? "opacity-50" : ""} ${!environment.is_global ? "cursor-move" : ""}`,
          onClick: () => handleSetActive(environment),
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
                !environment.is_global && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "cursor-move p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded", children: /* @__PURE__ */ jsxRuntimeExports.jsx(GripVertical, { className: "w-4 h-4 text-gray-400" }) }),
                environment.color && /* @__PURE__ */ jsxRuntimeExports.jsx(
                  "div",
                  {
                    className: "w-4 h-4 rounded-full border-2 border-white shadow-sm",
                    style: { backgroundColor: environment.color.hex }
                  }
                ),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsxs("h3", { className: "font-medium text-gray-900 dark:text-gray-100", children: [
                    environment.name,
                    environment.is_global && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "ml-2 text-xs text-gray-500 dark:text-gray-400", children: "(Global)" })
                  ] }),
                  environment.description && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: environment.description })
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-xs text-gray-500 dark:text-gray-400", children: [
                  environment.variable_count,
                  " vars"
                ] }),
                environment.active && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-2 h-2 bg-blue-500 rounded-full" })
              ] })
            ] }),
            variables && variables.variables.length > 0 && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-3 pt-3 border-t border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-wrap gap-1", children: [
              variables.variables.slice(0, 3).map((variable) => /* @__PURE__ */ jsxRuntimeExports.jsx(
                "span",
                {
                  className: "inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200",
                  children: variable.name
                },
                variable.name
              )),
              variables.variables.length > 3 && /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-xs text-gray-500 dark:text-gray-400", children: [
                "+",
                variables.variables.length - 3,
                " more"
              ] })
            ] }) })
          ]
        }
      ) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(ContextMenuContent, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(ContextMenuItem, { onClick: () => handleSetActive(environment), children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Play, { className: "w-4 h-4 mr-2" }),
          "Set as Active"
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(ContextMenuItem, { onClick: () => handleEdit(environment), children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "w-4 h-4 mr-2" }),
          "Edit Environment"
        ] }),
        !environment.is_global && /* @__PURE__ */ jsxRuntimeExports.jsxs(
          ContextMenuItem,
          {
            onClick: () => handleDelete(environment),
            className: "text-red-600 dark:text-red-400",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Trash2, { className: "w-4 h-4 mr-2" }),
              "Delete Environment"
            ]
          }
        )
      ] })
    ] });
  };
  if (isLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: "Environments" }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4", children: [...Array(3)].map((_, i) => /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "animate-pulse", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-24 bg-gray-200 dark:bg-gray-700 rounded-lg" }) }, i)) })
    ] });
  }
  if (error) {
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: "Environments" }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-8 text-red-600 dark:text-red-400", children: "Failed to load environments" })
    ] });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: "Environments" }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Dialog, { open: isCreateDialogOpen, onOpenChange: setIsCreateDialogOpen, children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTrigger, { asChild: true, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "w-4 h-4" }),
          "New Environment"
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(DialogHeader, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Create New Environment" }) }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: "Name" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  value: createForm.name,
                  onChange: (e) => setCreateForm((prev) => ({ ...prev, name: e.target.value })),
                  placeholder: "e.g., Development, Staging, Production"
                }
              )
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: "Description (Optional)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  value: createForm.description || "",
                  onChange: (e) => setCreateForm((prev) => ({ ...prev, description: e.target.value })),
                  placeholder: "Brief description of this environment"
                }
              )
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => setIsCreateDialogOpen(false), children: "Cancel" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: handleCreate, disabled: createEnvironment.isPending, children: createEnvironment.isPending ? "Creating..." : "Create Environment" })
          ] })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4", children: environments == null ? void 0 : environments.environments.sort((a, b) => {
      if (a.is_global && !b.is_global) return -1;
      if (!a.is_global && b.is_global) return 1;
      return (a.order || 0) - (b.order || 0);
    }).map((environment) => /* @__PURE__ */ jsxRuntimeExports.jsx(EnvironmentCard, { environment }, environment.id)) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open: !!editingEnvironment, onOpenChange: (open) => !open && setEditingEnvironment(null), children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogHeader, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Edit Environment" }) }),
      editingEnvironment && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: "Name" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              value: editForm.name || "",
              onChange: (e) => setEditForm((prev) => ({ ...prev, name: e.target.value }))
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", children: "Description (Optional)" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              value: editForm.description || "",
              onChange: (e) => setEditForm((prev) => ({ ...prev, description: e.target.value }))
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Color (Optional)" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex flex-wrap gap-2", children: PREDEFINED_COLORS.map((color) => /* @__PURE__ */ jsxRuntimeExports.jsx(
            "button",
            {
              onClick: () => setSelectedColor(color),
              className: `w-8 h-8 rounded-full border-2 ${(selectedColor == null ? void 0 : selectedColor.hex) === color.hex ? "border-gray-900 dark:border-gray-100" : "border-gray-300 dark:border-gray-600"}`,
              style: { backgroundColor: color.hex },
              title: color.name
            },
            color.hex
          )) }),
          selectedColor && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 mt-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "div",
              {
                className: "w-4 h-4 rounded-full border border-gray-300",
                style: { backgroundColor: selectedColor.hex }
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm text-gray-600 dark:text-gray-400", children: selectedColor.name })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => setEditingEnvironment(null), children: "Cancel" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: handleUpdate, disabled: updateEnvironment.isPending, children: updateEnvironment.isPending ? "Updating..." : "Update Environment" })
      ] })
    ] }) })
  ] });
}
function AutocompleteInput({
  value,
  onChange,
  onSelect,
  placeholder,
  className,
  workspaceId,
  context,
  disabled = false
}) {
  var _a;
  const [showSuggestions, setShowSuggestions] = reactExports.useState(false);
  const [selectedIndex, setSelectedIndex] = reactExports.useState(-1);
  const inputRef = reactExports.useRef(null);
  const suggestionsRef = reactExports.useRef(null);
  const autocomplete = useAutocomplete(workspaceId);
  const handleSuggestionSelect = reactExports.useCallback((suggestion) => {
    if (!autocomplete.data) return;
    const { start_position, end_position } = autocomplete.data;
    const beforeToken = value.slice(0, start_position);
    const afterToken = value.slice(end_position);
    const newValue = `${beforeToken}{{${suggestion.text}}}}${afterToken}`;
    const newCursorPosition = beforeToken.length + suggestion.text.length + 4;
    onChange(newValue);
    onSelect == null ? void 0 : onSelect(suggestion);
    setTimeout(() => {
      var _a2, _b;
      (_a2 = inputRef.current) == null ? void 0 : _a2.setSelectionRange(newCursorPosition, newCursorPosition);
      (_b = inputRef.current) == null ? void 0 : _b.focus();
    }, 0);
    setShowSuggestions(false);
    setSelectedIndex(-1);
  }, [value, onChange, onSelect, autocomplete.data]);
  const handleInputChange = reactExports.useCallback((e) => {
    const newValue = e.target.value;
    const newCursorPosition = e.target.selectionStart || 0;
    onChange(newValue);
    const textBeforeCursor = newValue.slice(0, newCursorPosition);
    const hasOpenBraces = textBeforeCursor.includes("{{");
    if (hasOpenBraces) {
      const lastOpenBrace = textBeforeCursor.lastIndexOf("{{");
      const textAfterOpenBrace = textBeforeCursor.slice(lastOpenBrace + 2);
      if (textAfterOpenBrace.length > 0 || textBeforeCursor.endsWith("{{")) {
        autocomplete.mutate({
          input: newValue,
          cursor_position: newCursorPosition,
          context
        });
        setShowSuggestions(true);
        setSelectedIndex(-1);
      } else {
        setShowSuggestions(false);
      }
    } else {
      setShowSuggestions(false);
    }
  }, [onChange, context, autocomplete]);
  const handleKeyDown = reactExports.useCallback((e) => {
    var _a2, _b, _c;
    if (!showSuggestions || !((_a2 = autocomplete.data) == null ? void 0 : _a2.suggestions.length)) {
      if (e.ctrlKey && e.key === " ") {
        e.preventDefault();
        const currentValue = ((_b = inputRef.current) == null ? void 0 : _b.value) || "";
        const currentCursorPosition = ((_c = inputRef.current) == null ? void 0 : _c.selectionStart) || 0;
        autocomplete.mutate({
          input: currentValue,
          cursor_position: currentCursorPosition,
          context
        });
        setShowSuggestions(true);
        setSelectedIndex(-1);
      }
      return;
    }
    const suggestions2 = autocomplete.data.suggestions;
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % suggestions2.length);
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => prev <= 0 ? suggestions2.length - 1 : prev - 1);
        break;
      case "Enter":
      case "Tab":
        if (selectedIndex >= 0) {
          e.preventDefault();
          handleSuggestionSelect(suggestions2[selectedIndex]);
        }
        break;
      case "Escape":
        setShowSuggestions(false);
        setSelectedIndex(-1);
        break;
    }
  }, [showSuggestions, selectedIndex, autocomplete.data, context, autocomplete, handleSuggestionSelect]);
  reactExports.useEffect(() => {
    const handleClickOutside = (event) => {
      if (suggestionsRef.current && !suggestionsRef.current.contains(event.target) && inputRef.current && !inputRef.current.contains(event.target)) {
        setShowSuggestions(false);
        setSelectedIndex(-1);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);
  const suggestions = ((_a = autocomplete.data) == null ? void 0 : _a.suggestions) || [];
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      "input",
      {
        ref: inputRef,
        type: "text",
        value,
        onChange: handleInputChange,
        onKeyDown: handleKeyDown,
        placeholder,
        className,
        disabled,
        autoComplete: "off",
        spellCheck: false
      }
    ),
    showSuggestions && suggestions.length > 0 && /* @__PURE__ */ jsxRuntimeExports.jsx(
      "div",
      {
        ref: suggestionsRef,
        className: "absolute z-50 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto",
        children: suggestions.map((suggestion, index) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "div",
          {
            className: `px-3 py-2 cursor-pointer border-b border-gray-100 dark:border-gray-700 last:border-b-0 ${index === selectedIndex ? "bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300" : "hover:bg-gray-50 dark:hover:bg-gray-700"}`,
            onClick: () => handleSuggestionSelect(suggestion),
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-between", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium text-gray-900 dark:text-gray-100", children: suggestion.text }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: `text-xs px-2 py-1 rounded ${suggestion.kind === "variable" ? "bg-green-100 dark:bg-green-900/20 text-green-700 dark:text-green-300" : "bg-blue-100 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300"}`, children: suggestion.kind })
              ] }) }),
              suggestion.documentation && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-gray-600 dark:text-gray-400 mt-1", children: suggestion.documentation })
            ]
          },
          `${suggestion.kind}-${suggestion.text}`
        ))
      }
    ),
    autocomplete.isPending && showSuggestions && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "absolute right-3 top-1/2 transform -translate-y-1/2", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600" }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "absolute right-3 top-1/2 transform -translate-y-1/2 text-xs text-gray-400 dark:text-gray-500 opacity-0 group-hover:opacity-100 transition-opacity", children: "Ctrl+Space" })
  ] });
}
function useRealityShortcuts({
  onOpenPresetManager,
  enabled = true,
  defaultLevel = 3
} = {}) {
  const setLevelMutation = useSetRealityLevel();
  const onOpenPresetManagerRef = reactExports.useRef(onOpenPresetManager);
  if (onOpenPresetManagerRef.current !== onOpenPresetManager) {
    onOpenPresetManagerRef.current = onOpenPresetManager;
  }
  const handleSetLevel = reactExports.useCallback(
    (level) => {
      if (setLevelMutation.isPending) return;
      const levelNames = [
        "Static Stubs",
        "Light Simulation",
        "Moderate Realism",
        "High Realism",
        "Production Chaos"
      ];
      setLevelMutation.mutate(level, {
        onSuccess: () => {
          ue.success(`Reality level set to ${level}: ${levelNames[level - 1]}`, {
            description: "Press Ctrl+Shift+R to reset to default"
          });
        },
        onError: (error) => {
          ue.error("Failed to set reality level", {
            description: error instanceof Error ? error.message : "Unknown error"
          });
        }
      });
    },
    [setLevelMutation]
  );
  const handleReset = reactExports.useCallback(() => {
    if (setLevelMutation.isPending) return;
    handleSetLevel(defaultLevel);
  }, [defaultLevel, handleSetLevel, setLevelMutation]);
  const handleOpenPresetManager = reactExports.useCallback(() => {
    if (onOpenPresetManagerRef.current) {
      onOpenPresetManagerRef.current();
    } else {
      ue.info("Preset manager not available", {
        description: "Navigate to Configuration > Reality Slider to manage presets"
      });
    }
  }, []);
  const shortcuts = [
    // Level 1: Static Stubs
    {
      key: "1",
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(1),
      description: "Set reality level to 1 (Static Stubs)",
      enabled
    },
    // Level 2: Light Simulation
    {
      key: "2",
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(2),
      description: "Set reality level to 2 (Light Simulation)",
      enabled
    },
    // Level 3: Moderate Realism
    {
      key: "3",
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(3),
      description: "Set reality level to 3 (Moderate Realism)",
      enabled
    },
    // Level 4: High Realism
    {
      key: "4",
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(4),
      description: "Set reality level to 4 (High Realism)",
      enabled
    },
    // Level 5: Production Chaos
    {
      key: "5",
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(5),
      description: "Set reality level to 5 (Production Chaos)",
      enabled
    },
    // Reset to default
    {
      key: "r",
      ctrl: true,
      shift: true,
      handler: handleReset,
      description: `Reset reality level to ${defaultLevel} (default)`,
      enabled
    },
    // Open preset manager
    {
      key: "p",
      ctrl: true,
      shift: true,
      handler: handleOpenPresetManager,
      description: "Open preset manager",
      enabled: enabled && !!onOpenPresetManager
    }
  ];
  useKeyboardNavigation({
    shortcuts,
    enabled
  });
  return {
    shortcuts: shortcuts.map((s) => ({
      key: s.key,
      modifiers: {
        ctrl: s.ctrl,
        shift: s.shift,
        alt: s.alt,
        meta: s.meta
      },
      description: s.description
    }))
  };
}
const Slider = reactExports.forwardRef(
  ({
    className,
    min = 0,
    max = 100,
    step = 1,
    value,
    onChange,
    unit,
    label,
    showValue = true,
    description,
    disabled,
    ...props
  }, ref) => {
    const [internalValue, setInternalValue] = reactExports.useState(value ?? min);
    reactExports.useEffect(() => {
      if (value !== void 0) {
        setInternalValue(value);
      }
    }, [value]);
    const handleChange = (e) => {
      const newValue = parseFloat(e.target.value);
      setInternalValue(newValue);
      onChange == null ? void 0 : onChange(newValue);
    };
    const displayValue = value !== void 0 ? value : internalValue;
    const percentage = (displayValue - min) / (max - min) * 100;
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "w-full space-y-2", children: [
      (label || showValue) && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
        label && /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-700 dark:text-gray-300", children: label }),
        showValue && /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm font-semibold text-gray-900 dark:text-gray-100 tabular-nums", children: [
          displayValue.toLocaleString(),
          unit && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "ml-1 text-gray-500 dark:text-gray-400", children: unit })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "relative flex items-center", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
        "input",
        {
          type: "range",
          ref,
          min,
          max,
          step,
          value: displayValue,
          onChange: handleChange,
          disabled,
          className: cn(
            "h-2 w-full appearance-none rounded-lg bg-gray-200 dark:bg-gray-700 outline-none transition-all",
            "disabled:opacity-50 disabled:cursor-not-allowed",
            // Webkit (Chrome, Safari, Edge)
            "[&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-blue-600 dark:[&::-webkit-slider-thumb]:bg-blue-500 [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:shadow-sm [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:hover:bg-blue-700 dark:[&::-webkit-slider-thumb]:hover:bg-blue-400 [&::-webkit-slider-thumb]:active:scale-110",
            // Firefox
            "[&::-moz-range-thumb]:h-4 [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-blue-600 dark:[&::-moz-range-thumb]:bg-blue-500 [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:shadow-sm [&::-moz-range-thumb]:transition-all [&::-moz-range-thumb]:hover:bg-blue-700 dark:[&::-moz-range-thumb]:hover:bg-blue-400",
            // Track fill (visual progress indicator)
            "before:absolute before:left-0 before:top-0 before:h-2 before:rounded-lg before:bg-blue-600 dark:before:bg-blue-500 before:pointer-events-none",
            className
          ),
          style: {
            // @ts-ignore - CSS custom property for track fill
            "--track-fill": `${percentage}%`,
            background: `linear-gradient(to right, rgb(37 99 235) 0%, rgb(37 99 235) var(--track-fill), rgb(229 231 235) var(--track-fill), rgb(229 231 235) 100%)`
          },
          ...props
        }
      ) }),
      description && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400", children: description }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between text-xs text-gray-400 dark:text-gray-500", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
          min,
          unit && ` ${unit}`
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
          max,
          unit && ` ${unit}`
        ] })
      ] })
    ] });
  }
);
Slider.displayName = "Slider";
function Card({ title, icon, children, className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "div",
    {
      className: cn(
        "bg-bg-primary border border-border rounded-xl shadow-sm",
        // subtle brand accent on the left edge to add color without overpowering
        "border-l-4 border-l-brand-200",
        "hover:shadow-lg hover:border-brand-200 transition-all duration-200 ease-out",
        "hover:-translate-y-0.5 group",
        className
      ),
      ...props,
      children: [
        title && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "border-b border-border/50 px-6 py-4 bg-brand-50 dark:bg-brand-900/10 rounded-t-xl", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 flex items-center gap-3", children: [
          icon && typeof title === "string" && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "p-1.5 rounded-lg bg-brand-50 text-brand-600 group-hover:bg-brand-100 transition-colors duration-200 dark:bg-brand-900/20 dark:text-brand-400", children: icon }),
          title
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("p-6", title ? "" : "pt-6"), children })
      ]
    }
  );
}
function CardHeader({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: cn("flex flex-col space-y-1.5 p-6", className),
      ...props
    }
  );
}
function CardTitle({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "h3",
    {
      className: cn("text-2xl font-semibold leading-none tracking-tight", className),
      ...props
    }
  );
}
function CardDescription({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "p",
    {
      className: cn("text-sm text-muted-foreground", className),
      ...props
    }
  );
}
function CardContent({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: cn("p-6 pt-0", className),
      ...props
    }
  );
}
function Badge({
  children,
  variant = "default",
  className,
  ...props
}) {
  const variantClasses = {
    default: "bg-muted text-muted-foreground",
    secondary: "bg-secondary text-secondary-foreground",
    success: "bg-success/15 text-success",
    warning: "bg-warning/15 text-warning",
    danger: "bg-danger/15 text-danger",
    destructive: "bg-red-100 text-red-700 dark:bg-red-900/20 dark:text-red-400",
    error: "bg-danger/15 text-danger",
    brand: "bg-brand/15 text-brand",
    info: "bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400",
    outline: "border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "span",
    {
      className: cn(
        "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium transition-colors",
        variantClasses[variant],
        className
      ),
      ...props,
      children
    }
  );
}
function Tooltip({ content, children }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative group inline-block", children: [
    children,
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "absolute bottom-full left-1/2 transform -translate-x-1/2 mb-2 px-2 py-1 text-xs text-white bg-gray-900 dark:bg-gray-700 rounded opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 whitespace-nowrap z-50 pointer-events-none", children: [
      content,
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "absolute top-full left-1/2 transform -translate-x-1/2 -mt-1 border-4 border-transparent border-t-gray-900 dark:border-t-gray-700" })
    ] })
  ] });
}
const REALITY_LEVELS = [
  {
    value: 1,
    name: "Static Stubs",
    description: "Simple, instant responses with no chaos",
    icon: Shield,
    color: "text-gray-500",
    bgColor: "bg-gray-100 dark:bg-gray-800",
    borderColor: "border-gray-300 dark:border-gray-700",
    features: ["No chaos", "0ms latency", "No AI"]
  },
  {
    value: 2,
    name: "Light Simulation",
    description: "Minimal latency, basic intelligence",
    icon: Activity,
    color: "text-blue-500",
    bgColor: "bg-blue-50 dark:bg-blue-900/20",
    borderColor: "border-blue-300 dark:border-blue-700",
    features: ["No chaos", "10-50ms latency", "Basic AI"]
  },
  {
    value: 3,
    name: "Moderate Realism",
    description: "Some chaos, moderate latency, full intelligence",
    icon: Gauge,
    color: "text-green-500",
    bgColor: "bg-green-50 dark:bg-green-900/20",
    borderColor: "border-green-300 dark:border-green-700",
    features: ["5% errors, 10% delays", "50-200ms latency", "Full AI"]
  },
  {
    value: 4,
    name: "High Realism",
    description: "Increased chaos, realistic latency, session state",
    icon: TriangleAlert,
    color: "text-orange-500",
    bgColor: "bg-orange-50 dark:bg-orange-900/20",
    borderColor: "border-orange-300 dark:border-orange-700",
    features: ["10% errors, 20% delays", "100-500ms latency", "AI + Sessions"]
  },
  {
    value: 5,
    name: "Production Chaos",
    description: "Maximum chaos, production-like latency, full features",
    icon: Zap,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-900/20",
    borderColor: "border-red-300 dark:border-red-700",
    features: ["15% errors, 30% delays", "200-2000ms latency", "Full AI + Mutations"]
  }
];
function RealitySlider({ className, compact = false }) {
  const { data: realityData, isLoading } = useRealityLevel();
  const setLevelMutation = useSetRealityLevel();
  const [localLevel, setLocalLevel] = reactExports.useState(3);
  const [isDragging, setIsDragging] = reactExports.useState(false);
  useRealityShortcuts({
    enabled: !compact
    // Only enable shortcuts in full mode
  });
  reactExports.useEffect(() => {
    if (realityData == null ? void 0 : realityData.level) {
      setLocalLevel(realityData.level);
    }
  }, [realityData]);
  const currentLevel = (realityData == null ? void 0 : realityData.level) ?? localLevel;
  const levelConfig = REALITY_LEVELS.find((l) => l.value === currentLevel) || REALITY_LEVELS[2];
  const Icon2 = levelConfig.icon;
  const commitTimerRef = React.useRef(null);
  const handleLevelChange = reactExports.useCallback((newLevel) => {
    setLocalLevel(newLevel);
    setIsDragging(true);
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
    }
    commitTimerRef.current = setTimeout(() => {
      setIsDragging(false);
      if (newLevel === currentLevel) return;
      const levelConfig2 = REALITY_LEVELS.find((l) => l.value === newLevel) || REALITY_LEVELS[2];
      setLevelMutation.mutate(newLevel, {
        onSuccess: () => {
          ue.success(`Reality level set to ${newLevel}: ${levelConfig2.name}`, {
            description: levelConfig2.description
          });
        },
        onError: (error) => {
          ue.error("Failed to set reality level", {
            description: error instanceof Error ? error.message : "Unknown error"
          });
          setLocalLevel(currentLevel);
        }
      });
    }, 300);
  }, [currentLevel, setLevelMutation]);
  reactExports.useEffect(() => {
    return () => {
      if (commitTimerRef.current) {
        clearTimeout(commitTimerRef.current);
      }
    };
  }, []);
  const handleLevelCommit = reactExports.useCallback((newLevel) => {
    if (commitTimerRef.current) {
      clearTimeout(commitTimerRef.current);
    }
    setIsDragging(false);
    if (newLevel === currentLevel) return;
    const levelConfig2 = REALITY_LEVELS.find((l) => l.value === newLevel) || REALITY_LEVELS[2];
    setLevelMutation.mutate(newLevel, {
      onSuccess: () => {
        ue.success(`Reality level set to ${newLevel}: ${levelConfig2.name}`, {
          description: levelConfig2.description
        });
      },
      onError: (error) => {
        ue.error("Failed to set reality level", {
          description: error instanceof Error ? error.message : "Unknown error"
        });
        setLocalLevel(currentLevel);
      }
    });
  }, [currentLevel, setLevelMutation]);
  const handleQuickSet = reactExports.useCallback((level) => {
    if (level === currentLevel) return;
    handleLevelCommit(level);
  }, [currentLevel, handleLevelCommit]);
  if (isLoading && !realityData) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { className: cn("p-6 animate-pulse", className), children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-20 bg-gray-200 dark:bg-gray-700 rounded-lg" }) });
  }
  if (compact) {
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn("flex items-center gap-3", className), children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: cn("h-5 w-5", levelConfig.color) }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm font-medium text-gray-700 dark:text-gray-300", children: [
          "Level ",
          currentLevel
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Slider,
        {
          min: 1,
          max: 5,
          step: 1,
          value: localLevel,
          onChange: handleLevelChange,
          className: "w-32",
          showValue: false
        }
      )
    ] });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    Card,
    {
      className: cn(
        "p-6 transition-all duration-300 ease-out",
        "hover:shadow-lg hover:-translate-y-0.5",
        levelConfig.bgColor,
        `border-2 ${levelConfig.borderColor}`,
        className
      ),
      children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start justify-between mb-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              "div",
              {
                className: cn(
                  "p-3 rounded-xl transition-all duration-200",
                  levelConfig.bgColor,
                  levelConfig.color
                ),
                children: /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: "h-6 w-6" })
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: "Reality Slider" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: levelConfig.name })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Badge,
            {
              variant: "default",
              className: cn("text-sm font-semibold", levelConfig.color),
              children: [
                "Level ",
                currentLevel
              ]
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "mb-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between mb-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "text-sm font-medium text-gray-700 dark:text-gray-300", children: "Realism Level" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-lg font-bold text-gray-900 dark:text-gray-100 tabular-nums", children: [
              localLevel,
              " / 5"
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Slider,
            {
              min: 1,
              max: 5,
              step: 1,
              value: localLevel,
              onChange: handleLevelChange,
              disabled: setLevelMutation.isPending,
              description: levelConfig.description
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-between mt-4 px-1", children: REALITY_LEVELS.map((level) => {
            const LevelIcon = level.icon;
            const isActive = level.value === currentLevel;
            const isHovered = level.value === localLevel && isDragging;
            return /* @__PURE__ */ jsxRuntimeExports.jsx(
              Tooltip,
              {
                content: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-semibold mb-1", children: level.name }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xs text-gray-300", children: level.description }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-2 text-xs", children: level.features.map((feature, idx) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                    "• ",
                    feature
                  ] }, idx)) })
                ] }),
                children: /* @__PURE__ */ jsxRuntimeExports.jsxs(
                  "button",
                  {
                    type: "button",
                    onClick: () => handleQuickSet(level.value),
                    disabled: setLevelMutation.isPending,
                    className: cn(
                      "flex flex-col items-center gap-1.5 p-2 rounded-lg transition-all duration-200",
                      "hover:bg-white/50 dark:hover:bg-gray-800/50",
                      "disabled:opacity-50 disabled:cursor-not-allowed",
                      isActive && "bg-white dark:bg-gray-800 shadow-sm",
                      isHovered && !isActive && "bg-white/30 dark:bg-gray-800/30"
                    ),
                    children: [
                      /* @__PURE__ */ jsxRuntimeExports.jsx(
                        LevelIcon,
                        {
                          className: cn(
                            "h-5 w-5 transition-all duration-200",
                            isActive ? level.color : "text-gray-400 dark:text-gray-500",
                            isHovered && !isActive && "scale-110"
                          )
                        }
                      ),
                      /* @__PURE__ */ jsxRuntimeExports.jsx(
                        "span",
                        {
                          className: cn(
                            "text-xs font-medium transition-all duration-200",
                            isActive ? "text-gray-900 dark:text-gray-100" : "text-gray-500 dark:text-gray-400"
                          ),
                          children: level.value
                        }
                      )
                    ]
                  }
                )
              },
              level.value
            );
          }) })
        ] }),
        realityData && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-6 p-4 rounded-lg bg-white/50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-3 gap-4 text-sm", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1", children: "Chaos" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm font-semibold text-gray-900 dark:text-gray-100", children: realityData.chaos.enabled ? /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
              Math.round(realityData.chaos.error_rate * 100),
              "% errors",
              /* @__PURE__ */ jsxRuntimeExports.jsx("br", {}),
              Math.round(realityData.chaos.delay_rate * 100),
              "% delays"
            ] }) : "Disabled" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1", children: "Latency" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-sm font-semibold text-gray-900 dark:text-gray-100", children: [
              realityData.latency.base_ms,
              "ms",
              realityData.latency.jitter_ms > 0 && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
                " ±",
                realityData.latency.jitter_ms,
                "ms"
              ] })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1", children: "MockAI" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm font-semibold text-gray-900 dark:text-gray-100", children: realityData.mockai.enabled ? /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-green-600 dark:text-green-400", children: "Enabled" }) : /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-gray-500", children: "Disabled" }) })
          ] })
        ] }) }),
        setLevelMutation.isPending && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "mt-4 flex items-center justify-center gap-2 text-sm text-gray-600 dark:text-gray-400", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Applying reality level..." })
        ] })
      ]
    }
  );
}
const REALITY_LEVEL_CONFIG = [
  {
    value: 1,
    name: "Static Stubs",
    icon: Shield,
    color: "text-gray-500",
    bgColor: "bg-gray-100 dark:bg-gray-800",
    borderColor: "border-gray-300 dark:border-gray-700"
  },
  {
    value: 2,
    name: "Light Simulation",
    icon: Activity,
    color: "text-blue-500",
    bgColor: "bg-blue-50 dark:bg-blue-900/20",
    borderColor: "border-blue-300 dark:border-blue-700"
  },
  {
    value: 3,
    name: "Moderate Realism",
    icon: Gauge,
    color: "text-green-500",
    bgColor: "bg-green-50 dark:bg-green-900/20",
    borderColor: "border-green-300 dark:border-green-700"
  },
  {
    value: 4,
    name: "High Realism",
    icon: TriangleAlert,
    color: "text-orange-500",
    bgColor: "bg-orange-50 dark:bg-orange-900/20",
    borderColor: "border-orange-300 dark:border-orange-700"
  },
  {
    value: 5,
    name: "Production Chaos",
    icon: Zap,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-900/20",
    borderColor: "border-red-300 dark:border-red-700"
  }
];
function RealityIndicator({
  className,
  showIcon = true,
  showLabel = false,
  variant = "default"
}) {
  const { data: realityData, isLoading } = useRealityLevel();
  if (isLoading || !realityData) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: "outline", className: cn("animate-pulse", className), children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-3 w-8 bg-gray-200 dark:bg-gray-700 rounded" }) });
  }
  const level = realityData.level;
  const levelConfig = REALITY_LEVEL_CONFIG.find((l) => l.value === level) || REALITY_LEVEL_CONFIG[2];
  const Icon2 = levelConfig.icon;
  const content = /* @__PURE__ */ jsxRuntimeExports.jsxs(
    Badge,
    {
      variant: "outline",
      className: cn(
        "flex items-center gap-1.5 transition-all duration-200",
        levelConfig.bgColor,
        levelConfig.borderColor,
        className
      ),
      children: [
        showIcon && /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: cn("h-3.5 w-3.5", levelConfig.color) }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: cn("font-semibold tabular-nums", levelConfig.color), children: variant === "minimal" ? level : `L${level}` }),
        showLabel && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: cn("text-xs", levelConfig.color), children: levelConfig.name })
      ]
    }
  );
  if (variant === "minimal") {
    return content;
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    Tooltip,
    {
      content: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "font-semibold mb-1", children: [
          "Reality Level ",
          level,
          ": ",
          levelConfig.name
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xs text-gray-300", children: realityData.description }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "mt-2 text-xs space-y-1", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Chaos:" }),
            " ",
            realityData.chaos.enabled ? `${Math.round(realityData.chaos.error_rate * 100)}% errors, ${Math.round(realityData.chaos.delay_rate * 100)}% delays` : "Disabled"
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Latency:" }),
            " ",
            realityData.latency.base_ms,
            "ms",
            realityData.latency.jitter_ms > 0 && ` ±${realityData.latency.jitter_ms}ms`
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "MockAI:" }),
            " ",
            realityData.mockai.enabled ? "Enabled" : "Disabled"
          ] })
        ] })
      ] }),
      children: content
    }
  );
}
const Label = reactExports.forwardRef(
  ({ className, children, required, ...props }, ref) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "label",
    {
      ref,
      className: cn(
        "text-sm font-medium leading-none text-gray-900 dark:text-gray-100 peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
        className
      ),
      ...props,
      children: [
        children,
        required && /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-red-500 ml-0.5", "aria-hidden": "true", children: "*" })
      ]
    }
  )
);
Label.displayName = "Label";
const Textarea = reactExports.forwardRef(
  ({ className, error, errorId, "aria-invalid": ariaInvalid, "aria-describedby": ariaDescribedby, ...props }, ref) => {
    const hasError = !!error || ariaInvalid === true || ariaInvalid === "true";
    const describedBy = [ariaDescribedby, errorId].filter(Boolean).join(" ") || void 0;
    return /* @__PURE__ */ jsxRuntimeExports.jsx(
      "textarea",
      {
        className: cn(
          "flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          hasError && "border-red-500 focus-visible:ring-red-500",
          className
        ),
        ref,
        "aria-invalid": hasError || void 0,
        "aria-describedby": describedBy,
        ...props
      }
    );
  }
);
Textarea.displayName = "Textarea";
function RealityPresetManager({ className }) {
  const { data: presets, isLoading: presetsLoading } = useRealityPresets();
  const importMutation = useImportRealityPreset();
  const exportMutation = useExportRealityPreset();
  const [exportDialogOpen, setExportDialogOpen] = reactExports.useState(false);
  const [importDialogOpen, setImportDialogOpen] = reactExports.useState(false);
  const [presetName, setPresetName] = reactExports.useState("");
  const [presetDescription, setPresetDescription] = reactExports.useState("");
  const [selectedPresetPath, setSelectedPresetPath] = reactExports.useState("");
  const handleExport = () => {
    if (!presetName.trim()) {
      ue.error("Preset name is required");
      return;
    }
    exportMutation.mutate(
      {
        name: presetName.trim(),
        description: presetDescription.trim() || void 0
      },
      {
        onSuccess: (data) => {
          ue.success("Preset exported successfully", {
            description: `Saved to ${data.path}`
          });
          setExportDialogOpen(false);
          setPresetName("");
          setPresetDescription("");
        },
        onError: (error) => {
          ue.error("Failed to export preset", {
            description: error instanceof Error ? error.message : "Unknown error"
          });
        }
      }
    );
  };
  const handleImport = (path) => {
    importMutation.mutate(path, {
      onSuccess: (data) => {
        ue.success("Preset imported successfully", {
          description: `Applied ${data.name} (Level ${data.level}: ${data.level_name})`
        });
        setImportDialogOpen(false);
        setSelectedPresetPath("");
      },
      onError: (error) => {
        ue.error("Failed to import preset", {
          description: error instanceof Error ? error.message : "Unknown error"
        });
      }
    });
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: cn("p-6", className), children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { className: "text-lg font-semibold text-gray-900 dark:text-gray-100", children: "Reality Presets" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { className: "text-sm text-gray-600 dark:text-gray-400", children: "Save and load reality level configurations for different testing scenarios" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Dialog, { open: exportDialogOpen, onOpenChange: setExportDialogOpen, children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTrigger, { asChild: true, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              variant: "default",
              className: "flex items-center gap-2",
              disabled: exportMutation.isPending,
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Download, { className: "h-4 w-4" }),
                "Export Current"
              ]
            }
          ) }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Export Reality Preset" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { children: "Save the current reality level configuration as a preset for later use" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4 py-4", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { htmlFor: "preset-name", children: "Preset Name *" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Input$1,
                  {
                    id: "preset-name",
                    value: presetName,
                    onChange: (e) => setPresetName(e.target.value),
                    placeholder: "e.g., production-chaos, staging-realistic",
                    className: "mt-1"
                  }
                )
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { htmlFor: "preset-description", children: "Description (Optional)" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Textarea,
                  {
                    id: "preset-description",
                    value: presetDescription,
                    onChange: (e) => setPresetDescription(e.target.value),
                    placeholder: "Describe when to use this preset...",
                    className: "mt-1",
                    rows: 3
                  }
                )
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Button$1,
                {
                  variant: "outline",
                  onClick: () => {
                    setExportDialogOpen(false);
                    setPresetName("");
                    setPresetDescription("");
                  },
                  children: "Cancel"
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Button$1,
                {
                  onClick: handleExport,
                  disabled: !presetName.trim() || exportMutation.isPending,
                  children: exportMutation.isPending ? "Exporting..." : "Export Preset"
                }
              )
            ] })
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Dialog, { open: importDialogOpen, onOpenChange: setImportDialogOpen, children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTrigger, { asChild: true, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              variant: "outline",
              className: "flex items-center gap-2",
              disabled: importMutation.isPending,
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Upload, { className: "h-4 w-4" }),
                "Import Preset"
              ]
            }
          ) }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Import Reality Preset" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { children: "Load a previously saved reality level configuration" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4 py-4", children: presetsLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-center py-8", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-6 w-6 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" }) }) : presets && presets.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2 max-h-64 overflow-y-auto", children: presets.map((preset) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
              "button",
              {
                type: "button",
                onClick: () => {
                  setSelectedPresetPath(preset.path);
                  handleImport(preset.path);
                },
                disabled: importMutation.isPending,
                className: cn(
                  "w-full text-left p-3 rounded-lg border transition-all duration-200",
                  "hover:bg-gray-50 dark:hover:bg-gray-800",
                  "hover:border-gray-300 dark:hover:border-gray-600",
                  "disabled:opacity-50 disabled:cursor-not-allowed",
                  selectedPresetPath === preset.path ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20" : "border-gray-200 dark:border-gray-700"
                ),
                children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
                      /* @__PURE__ */ jsxRuntimeExports.jsx(FileText, { className: "h-4 w-4 text-gray-500 dark:text-gray-400" }),
                      /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium text-gray-900 dark:text-gray-100", children: preset.name })
                    ] }),
                    selectedPresetPath === preset.path && importMutation.isPending && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" }),
                    selectedPresetPath === preset.path && !importMutation.isPending && /* @__PURE__ */ jsxRuntimeExports.jsx(Check, { className: "h-4 w-4 text-green-500" })
                  ] }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: preset.path })
                ]
              },
              preset.id
            )) }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Alert, { variant: "info", className: "mt-4", children: /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm", children: "No presets available. Export a preset to get started." }) }) }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(DialogFooter, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(
              Button$1,
              {
                variant: "outline",
                onClick: () => {
                  setImportDialogOpen(false);
                  setSelectedPresetPath("");
                },
                children: "Close"
              }
            ) })
          ] })
        ] })
      ] }),
      presetsLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-center py-8", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-6 w-6 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" }) }) : presets && presets.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("h4", { className: "text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: [
          "Available Presets (",
          presets.length,
          ")"
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2 max-h-64 overflow-y-auto", children: presets.map((preset) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "div",
          {
            className: "flex items-center justify-between p-3 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 flex-1 min-w-0", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(FileText, { className: "h-4 w-4 text-gray-500 dark:text-gray-400 flex-shrink-0" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex-1 min-w-0", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm font-medium text-gray-900 dark:text-gray-100 truncate", children: preset.name }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 truncate", children: preset.path })
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs(
                Button$1,
                {
                  variant: "ghost",
                  size: "sm",
                  onClick: () => handleImport(preset.path),
                  disabled: importMutation.isPending,
                  className: "flex-shrink-0",
                  children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsx(Upload, { className: "h-4 w-4 mr-1" }),
                    "Load"
                  ]
                }
              )
            ]
          },
          preset.id
        )) })
      ] }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Alert, { variant: "info", children: /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm", children: "No presets saved yet. Export your current configuration to create one." }) }),
      (importMutation.isPending || exportMutation.isPending) && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-center gap-2 text-sm text-gray-600 dark:text-gray-400 py-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600 dark:border-gray-600 dark:border-t-gray-300" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: importMutation.isPending ? "Importing preset..." : "Exporting preset..." })
      ] })
    ] })
  ] });
}
function extractPort(address) {
  if (!address) return "";
  const parts = address.split(":");
  return parts[parts.length - 1] || "";
}
function isValidUrl(url) {
  if (!url) return true;
  try {
    const urlObj = new URL(url);
    return urlObj.protocol === "http:" || urlObj.protocol === "https:";
  } catch {
    return false;
  }
}
function isValidPort(port) {
  return port >= 1 && port <= 65535;
}
function ConfigPage() {
  const { t } = useI18n();
  const [activeSection, setActiveSection] = reactExports.useState("general");
  const { activeWorkspace } = useWorkspaceStore();
  const workspaceId = (activeWorkspace == null ? void 0 : activeWorkspace.id) || "default-workspace";
  const [hasUnsavedChanges, setHasUnsavedChanges] = reactExports.useState(false);
  const [showRestartDialog, setShowRestartDialog] = reactExports.useState(false);
  const [hasPendingPortConfig, setHasPendingPortConfig] = reactExports.useState(
    () => Boolean(localStorage.getItem("mockforge_pending_port_config"))
  );
  useRealityShortcuts({
    onOpenPresetManager: () => {
      setActiveSection("reality");
      setTimeout(() => {
        const presetSection = document.querySelector('[data-section="reality"]');
        if (presetSection) {
          presetSection.scrollIntoView({ behavior: "smooth", block: "start" });
        }
      }, 100);
    }
  });
  const { data: config, isLoading: configLoading } = useConfig();
  const { data: validation, isLoading: validationLoading } = useValidation();
  const { data: serverInfo, isLoading: serverInfoLoading } = useServerInfo();
  const updateLatency = useUpdateLatency();
  const updateFaults = useUpdateFaults();
  const updateProxy = useUpdateProxy();
  const updateValidation = useUpdateValidation();
  const restartServers = useRestartServers();
  const { data: restartStatus } = useRestartStatus();
  const [formData, setFormData] = reactExports.useState({
    general: {
      http_port: 3e3,
      ws_port: 3001,
      grpc_port: 50051,
      admin_port: 9080,
      ai_mode: "live"
    },
    restartInProgress: false,
    latency: { base_ms: 0, jitter_ms: 0 },
    faults: { enabled: false, failure_rate: 0, status_codes: [] },
    trafficShaping: {
      enabled: false,
      bandwidth: {
        enabled: false,
        max_bytes_per_sec: 1048576,
        // 1 MB/s
        burst_capacity_bytes: 10485760
        // 10 MB
      },
      burstLoss: {
        enabled: false,
        burst_probability: 0.1,
        burst_duration_ms: 5e3,
        loss_rate_during_burst: 0.5,
        recovery_time_ms: 3e4
      }
    },
    proxy: { enabled: false, upstream_url: "", timeout_seconds: 30 },
    validation: {
      mode: "enforce",
      aggregate_errors: true,
      validate_responses: true,
      overrides: {}
    },
    protocols: {
      http: true,
      graphql: true,
      grpc: true,
      websocket: true,
      smtp: false,
      mqtt: false,
      ftp: false,
      kafka: false,
      rabbitmq: false,
      amqp: false
    },
    templateTest: ""
  });
  const savePortConfig = (ports) => {
    localStorage.setItem("mockforge_pending_port_config", JSON.stringify(ports));
  };
  reactExports.useEffect(() => {
    const pendingConfig = localStorage.getItem("mockforge_pending_port_config");
    if (pendingConfig) {
      try {
        const ports = JSON.parse(pendingConfig);
        setFormData((prev) => ({
          ...prev,
          general: { ...prev.general, ...ports }
        }));
        setHasPendingPortConfig(true);
      } catch (error) {
        logger.error("Failed to parse pending port config", error);
        localStorage.removeItem("mockforge_pending_port_config");
        setHasPendingPortConfig(false);
      }
    }
  }, []);
  reactExports.useEffect(() => {
    if (restartStatus && formData.restartInProgress) {
      if (!restartStatus.restarting) {
        setFormData((prev) => ({ ...prev, restartInProgress: false }));
        ue.success("Server restarted successfully! Port configuration applied.");
        localStorage.removeItem("mockforge_pending_port_config");
        setHasPendingPortConfig(false);
      }
    }
  }, [restartStatus, formData.restartInProgress]);
  reactExports.useEffect(() => {
    if (config == null ? void 0 : config.latency) {
      setFormData((prev) => ({
        ...prev,
        latency: {
          base_ms: config.latency.base_ms,
          jitter_ms: config.latency.jitter_ms
        }
      }));
    }
    if (config == null ? void 0 : config.faults) {
      setFormData((prev) => ({
        ...prev,
        faults: {
          enabled: config.faults.enabled,
          failure_rate: config.faults.failure_rate,
          status_codes: config.faults.status_codes
        }
      }));
    }
    if (config == null ? void 0 : config.proxy) {
      setFormData((prev) => ({
        ...prev,
        proxy: {
          enabled: config.proxy.enabled,
          upstream_url: config.proxy.upstream_url || "",
          timeout_seconds: config.proxy.timeout_seconds
        }
      }));
    }
  }, [config]);
  reactExports.useEffect(() => {
    if (serverInfo && !hasPendingPortConfig) {
      setFormData((prev) => ({
        ...prev,
        general: {
          http_port: parseInt(extractPort(serverInfo.http_server)) || 3e3,
          ws_port: parseInt(extractPort(serverInfo.ws_server)) || 3001,
          grpc_port: parseInt(extractPort(serverInfo.grpc_server)) || 50051,
          admin_port: serverInfo.admin_port || 9080
        }
      }));
    }
  }, [serverInfo, hasPendingPortConfig]);
  reactExports.useEffect(() => {
    if (validation) {
      setFormData((prev) => ({
        ...prev,
        validation: {
          mode: validation.mode,
          aggregate_errors: validation.aggregate_errors,
          validate_responses: validation.validate_responses,
          overrides: validation.overrides
        }
      }));
    }
  }, [validation]);
  reactExports.useEffect(() => {
    let hasChanges = false;
    if (serverInfo) {
      const currentHttpPort = parseInt(extractPort(serverInfo.http_server)) || 3e3;
      const currentWsPort = parseInt(extractPort(serverInfo.ws_server)) || 3001;
      const currentGrpcPort = parseInt(extractPort(serverInfo.grpc_server)) || 50051;
      const currentAdminPort = serverInfo.admin_port || 9080;
      if (formData.general.http_port !== currentHttpPort || formData.general.ws_port !== currentWsPort || formData.general.grpc_port !== currentGrpcPort || formData.general.admin_port !== currentAdminPort) {
        hasChanges = true;
      }
    }
    if (config == null ? void 0 : config.latency) {
      if (formData.latency.base_ms !== config.latency.base_ms || formData.latency.jitter_ms !== config.latency.jitter_ms) {
        hasChanges = true;
      }
    }
    if (config == null ? void 0 : config.faults) {
      if (formData.faults.enabled !== config.faults.enabled || formData.faults.failure_rate !== config.faults.failure_rate || JSON.stringify(formData.faults.status_codes) !== JSON.stringify(config.faults.status_codes)) {
        hasChanges = true;
      }
    }
    if (config == null ? void 0 : config.proxy) {
      if (formData.proxy.enabled !== config.proxy.enabled || formData.proxy.upstream_url !== (config.proxy.upstream_url || "") || formData.proxy.timeout_seconds !== config.proxy.timeout_seconds) {
        hasChanges = true;
      }
    }
    if (validation) {
      if (formData.validation.mode !== validation.mode || formData.validation.aggregate_errors !== validation.aggregate_errors || formData.validation.validate_responses !== validation.validate_responses) {
        hasChanges = true;
      }
    }
    setHasUnsavedChanges(hasChanges);
  }, [formData, config, validation, serverInfo]);
  reactExports.useEffect(() => {
    const handleBeforeUnload = (e) => {
      if (hasUnsavedChanges) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [hasUnsavedChanges]);
  const handleSave = async (section) => {
    if (section === "proxy" && formData.proxy.enabled) {
      if (!formData.proxy.upstream_url) {
        ue.error("Upstream URL is required when proxy is enabled");
        return;
      }
      if (!isValidUrl(formData.proxy.upstream_url)) {
        ue.error("Invalid upstream URL. Must be a valid HTTP or HTTPS URL");
        return;
      }
      if (!isValidPort(formData.proxy.timeout_seconds)) {
        ue.error("Invalid timeout. Must be between 1 and 300 seconds");
        return;
      }
    }
    if (section === "general") {
      if (!isValidPort(formData.general.http_port)) {
        ue.error("Invalid HTTP port. Must be between 1 and 65535");
        return;
      }
      if (!isValidPort(formData.general.ws_port)) {
        ue.error("Invalid WebSocket port. Must be between 1 and 65535");
        return;
      }
      if (!isValidPort(formData.general.grpc_port)) {
        ue.error("Invalid gRPC port. Must be between 1 and 65535");
        return;
      }
      if (!isValidPort(formData.general.admin_port)) {
        ue.error("Invalid Admin port. Must be between 1 and 65535");
        return;
      }
    }
    try {
      switch (section) {
        case "latency":
          await updateLatency.mutateAsync({
            name: "default",
            base_ms: formData.latency.base_ms,
            jitter_ms: formData.latency.jitter_ms,
            tag_overrides: {}
          });
          ue.success("Latency configuration saved successfully");
          break;
        case "faults":
          await updateFaults.mutateAsync({
            enabled: formData.faults.enabled,
            failure_rate: formData.faults.failure_rate,
            status_codes: formData.faults.status_codes,
            active_failures: 0
          });
          ue.success("Fault injection configuration saved successfully");
          break;
        case "proxy":
          await updateProxy.mutateAsync({
            enabled: formData.proxy.enabled,
            upstream_url: formData.proxy.upstream_url,
            timeout_seconds: formData.proxy.timeout_seconds,
            requests_proxied: 0
          });
          ue.success("Proxy configuration saved successfully");
          break;
        case "validation":
          await updateValidation.mutateAsync({
            mode: formData.validation.mode,
            aggregate_errors: formData.validation.aggregate_errors,
            validate_responses: formData.validation.validate_responses,
            overrides: formData.validation.overrides
          });
          ue.success("Validation settings saved successfully");
          break;
        case "general": {
          savePortConfig(formData.general);
          setShowRestartDialog(true);
          break;
        }
        case "traffic-shaping":
          try {
            const response = await authenticatedFetch("/__mockforge/config/traffic-shaping", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(formData.trafficShaping)
            });
            if (!response.ok) {
              throw new Error(`HTTP error! status: ${response.status}`);
            }
            ue.success("Traffic shaping configuration saved successfully");
          } catch (error) {
            logger.error("Error saving traffic shaping", error);
            ue.error("Failed to save traffic shaping configuration");
          }
          break;
        default:
          ue.error(`Unknown section: ${section}`);
      }
    } catch (error) {
      logger.error(`Error saving ${section} configuration:`, error);
      ue.error(`Failed to save ${section} configuration`);
    }
  };
  const handleConfirmRestart = async () => {
    setShowRestartDialog(false);
    try {
      setFormData((prev) => ({ ...prev, restartInProgress: true }));
      ue.info("Saving configuration and restarting server...");
      await restartServers.mutateAsync("Port configuration updated");
    } catch (error) {
      setFormData((prev) => ({ ...prev, restartInProgress: false }));
      ue.error("Failed to restart server. Please restart manually.");
      logger.error("Server restart failed", error);
    }
  };
  const handleCancelRestart = () => {
    setShowRestartDialog(false);
    ue.info("Configuration saved locally. Restart the server manually to apply changes.");
  };
  const handleReset = (section) => {
    switch (section) {
      case "general":
        if (serverInfo) {
          setFormData((prev) => ({
            ...prev,
            general: {
              http_port: parseInt(extractPort(serverInfo.http_server)) || 3e3,
              ws_port: parseInt(extractPort(serverInfo.ws_server)) || 3001,
              grpc_port: parseInt(extractPort(serverInfo.grpc_server)) || 50051,
              admin_port: serverInfo.admin_port || 9080
            }
          }));
          ue.info("General settings reset to server values");
        }
        break;
      case "latency":
        if (config == null ? void 0 : config.latency) {
          setFormData((prev) => ({
            ...prev,
            latency: {
              base_ms: config.latency.base_ms,
              jitter_ms: config.latency.jitter_ms
            }
          }));
          ue.info("Latency configuration reset to server values");
        }
        break;
      case "faults":
        if (config == null ? void 0 : config.faults) {
          setFormData((prev) => ({
            ...prev,
            faults: {
              enabled: config.faults.enabled,
              failure_rate: config.faults.failure_rate,
              status_codes: config.faults.status_codes
            }
          }));
          ue.info("Fault injection configuration reset to server values");
        }
        break;
      case "proxy":
        if (config == null ? void 0 : config.proxy) {
          setFormData((prev) => ({
            ...prev,
            proxy: {
              enabled: config.proxy.enabled,
              upstream_url: config.proxy.upstream_url || "",
              timeout_seconds: config.proxy.timeout_seconds
            }
          }));
          ue.info("Proxy configuration reset to server values");
        }
        break;
      case "validation":
        if (validation) {
          setFormData((prev) => ({
            ...prev,
            validation: {
              mode: validation.mode,
              aggregate_errors: validation.aggregate_errors,
              validate_responses: validation.validate_responses,
              overrides: validation.overrides
            }
          }));
          ue.info("Validation settings reset to server values");
        }
        break;
      case "traffic-shaping":
        setFormData((prev) => ({
          ...prev,
          trafficShaping: {
            enabled: false,
            bandwidth: {
              enabled: false,
              max_bytes_per_sec: 1048576,
              burst_capacity_bytes: 10485760
            },
            burstLoss: {
              enabled: false,
              burst_probability: 0.1,
              burst_duration_ms: 5e3,
              loss_rate_during_burst: 0.5,
              recovery_time_ms: 3e4
            }
          }
        }));
        ue.info("Traffic shaping configuration reset to defaults");
        break;
      default:
        ue.error(`Unknown section: ${section}`);
    }
  };
  const handleResetAll = () => {
    handleReset("general");
    handleReset("latency");
    handleReset("faults");
    handleReset("traffic-shaping");
    handleReset("proxy");
    handleReset("validation");
    ue.success("All settings reset to server values");
  };
  const handleSaveAll = async () => {
    const sections2 = ["general", "latency", "faults", "traffic-shaping", "proxy", "validation"];
    let successCount = 0;
    let errorCount = 0;
    for (const section of sections2) {
      try {
        await handleSave(section);
        successCount++;
      } catch (_error) {
        errorCount++;
      }
    }
    if (errorCount === 0) {
      ue.success("All settings saved successfully");
    } else {
      ue.warning(`Saved ${successCount} sections, ${errorCount} failed`);
    }
  };
  if (configLoading || validationLoading || serverInfoLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        PageHeader,
        {
          title: t("page.config.title"),
          subtitle: t("page.config.subtitle")
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-center py-12", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "ml-3 text-lg text-gray-600 dark:text-gray-400", children: "Loading configuration..." })
      ] })
    ] });
  }
  const sections = [
    { id: "reality", label: "Reality Slider", icon: Zap, description: "Unified realism control" },
    { id: "general", label: "General", icon: Settings, description: "Basic MockForge settings" },
    { id: "protocols", label: "Protocols", icon: Server, description: "Protocol enable/disable settings" },
    { id: "latency", label: "Latency", icon: Zap, description: "Response delay and timing" },
    { id: "faults", label: "Fault Injection", icon: Shield, description: "Error simulation and failure modes" },
    { id: "traffic-shaping", label: "Traffic Shaping", icon: Wifi, description: "Bandwidth control and network simulation" },
    { id: "proxy", label: "Proxy", icon: Server, description: "Upstream proxy configuration" },
    { id: "validation", label: "Validation", icon: Database, description: "Request/response validation" },
    { id: "environment", label: "Environment", icon: Settings, description: "Environment variables" }
  ];
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-8", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      PageHeader,
      {
        title: t("page.config.title"),
        subtitle: hasUnsavedChanges ? "⚠️ You have unsaved changes" : t("page.config.subtitle"),
        action: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(RealityIndicator, {}),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              variant: "outline",
              size: "sm",
              className: "flex items-center gap-2",
              onClick: handleResetAll,
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4" }),
                "Reset All"
              ]
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              variant: "default",
              size: "sm",
              className: "flex items-center gap-2",
              onClick: handleSaveAll,
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Save, { className: "h-4 w-4" }),
                "Save All Changes"
              ]
            }
          )
        ] })
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 lg:grid-cols-4 gap-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "lg:col-span-1", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsx("nav", { className: "space-y-2", children: sections.map((section) => {
        const Icon2 = section.icon;
        return /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "button",
          {
            onClick: () => setActiveSection(section.id),
            className: `w-full flex items-center gap-3 px-3 py-3 rounded-lg text-left transition-colors ${activeSection === section.id ? "bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300" : "hover:bg-gray-50 dark:hover:bg-gray-800/50 text-gray-700 dark:text-gray-300"}`,
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Icon2, { className: "h-5 w-5" }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: section.label }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xs opacity-75", children: section.description })
              ] })
            ]
          },
          section.id
        );
      }) }) }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "lg:col-span-3", children: [
        activeSection === "reality" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Reality Slider", subtitle: "Unified control for chaos, latency, and MockAI", "data-section": "reality", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(RealitySlider, {}),
          /* @__PURE__ */ jsxRuntimeExports.jsx(RealityPresetManager, {}),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "mt-4 p-4 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "text-sm font-semibold text-blue-900 dark:text-blue-100 mb-2", children: "Keyboard Shortcuts" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-xs text-blue-800 dark:text-blue-200 space-y-1", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("kbd", { className: "px-1.5 py-0.5 bg-white dark:bg-gray-800 rounded border border-blue-300 dark:border-blue-700", children: "Ctrl+Shift+1-5" }),
                " Set reality level"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("kbd", { className: "px-1.5 py-0.5 bg-white dark:bg-gray-800 rounded border border-blue-300 dark:border-blue-700", children: "Ctrl+Shift+R" }),
                " Reset to default (Level 3)"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("kbd", { className: "px-1.5 py-0.5 bg-white dark:bg-gray-800 rounded border border-blue-300 dark:border-blue-700", children: "Ctrl+Shift+P" }),
                " Open preset manager"
              ] })
            ] })
          ] })
        ] }) }),
        activeSection === "general" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "General Settings", subtitle: "Basic MockForge configuration", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Server Configuration" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs text-gray-500 dark:text-gray-400 mb-1", children: "HTTP Port" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Input$1,
                  {
                    type: "number",
                    min: "1",
                    max: "65535",
                    value: formData.general.http_port,
                    onChange: (e) => setFormData((prev) => ({
                      ...prev,
                      general: { ...prev.general, http_port: parseInt(e.target.value) || 3e3 }
                    }))
                  }
                )
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs text-gray-500 dark:text-gray-400 mb-1", children: "WebSocket Port" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Input$1,
                  {
                    type: "number",
                    min: "1",
                    max: "65535",
                    value: formData.general.ws_port,
                    onChange: (e) => setFormData((prev) => ({
                      ...prev,
                      general: { ...prev.general, ws_port: parseInt(e.target.value) || 3001 }
                    }))
                  }
                )
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs text-gray-500 dark:text-gray-400 mb-1", children: "gRPC Port" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Input$1,
                  {
                    type: "number",
                    min: "1",
                    max: "65535",
                    value: formData.general.grpc_port,
                    onChange: (e) => setFormData((prev) => ({
                      ...prev,
                      general: { ...prev.general, grpc_port: parseInt(e.target.value) || 50051 }
                    }))
                  }
                )
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs text-gray-500 dark:text-gray-400 mb-1", children: "Admin Port" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Input$1,
                  {
                    type: "number",
                    min: "1",
                    max: "65535",
                    value: formData.general.admin_port,
                    onChange: (e) => setFormData((prev) => ({
                      ...prev,
                      general: { ...prev.general, admin_port: parseInt(e.target.value) || 9080 }
                    }))
                  }
                )
              ] })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "AI Mode" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-sm text-gray-600 dark:text-gray-400 mb-4", children: [
                "Control how AI-generated artifacts are used at runtime. In ",
                /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Generate Once Freeze" }),
                " mode, AI is only used to produce config/templates, and runtime mocks use frozen artifacts (no LLM calls). In ",
                /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Live" }),
                " mode, AI is used dynamically at runtime for each request."
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs(
                Select,
                {
                  value: formData.general.ai_mode || "live",
                  onValueChange: (value) => {
                    setFormData((prev) => ({
                      ...prev,
                      general: { ...prev.general, ai_mode: value }
                    }));
                    setHasUnsavedChanges(true);
                  },
                  children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsx(SelectTrigger, { className: "w-full", children: /* @__PURE__ */ jsxRuntimeExports.jsx(SelectValue, {}) }),
                    /* @__PURE__ */ jsxRuntimeExports.jsxs(SelectContent, { children: [
                      /* @__PURE__ */ jsxRuntimeExports.jsx(SelectItem, { value: "live", children: "Live - AI used dynamically at runtime" }),
                      /* @__PURE__ */ jsxRuntimeExports.jsx(SelectItem, { value: "generate_once_freeze", children: "Generate Once Freeze - Use frozen artifacts only" })
                    ] })
                  ]
                }
              ),
              formData.general.ai_mode === "generate_once_freeze" && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-3 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-xs text-blue-800 dark:text-blue-200", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("strong", { children: "Note:" }),
                " In this mode, AI-generated scenarios and personas will use frozen artifacts. Make sure to freeze your AI-generated artifacts before using them in this mode."
              ] }) })
            ] }) })
          ] }),
          formData.restartInProgress && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg mb-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "w-4 h-4 animate-spin text-blue-600" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm text-blue-700 dark:text-blue-300", children: "Server restart in progress... Configuration will be applied shortly." })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => handleReset("general"), disabled: formData.restartInProgress, children: "Reset" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: () => handleSave("general"), disabled: formData.restartInProgress, children: formData.restartInProgress ? /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "w-4 h-4 mr-2 animate-spin" }),
              "Restarting..."
            ] }) : "Save & Restart Server" })
          ] })
        ] }) }) }),
        activeSection === "protocols" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Protocol Configuration", subtitle: "Enable or disable protocol support", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6", children: [
          { key: "http", label: "HTTP/REST", description: "RESTful API mocking" },
          { key: "graphql", label: "GraphQL", description: "GraphQL API mocking" },
          { key: "grpc", label: "gRPC", description: "gRPC service mocking" },
          { key: "websocket", label: "WebSocket", description: "WebSocket connection mocking" },
          { key: "smtp", label: "SMTP", description: "Email protocol mocking" },
          { key: "mqtt", label: "MQTT", description: "IoT messaging protocol" },
          { key: "ftp", label: "FTP", description: "File transfer protocol" },
          { key: "kafka", label: "Kafka", description: "Event streaming platform" },
          { key: "rabbitmq", label: "RabbitMQ", description: "Message queuing system" },
          { key: "amqp", label: "AMQP", description: "Advanced message queuing" }
        ].map((protocol) => {
          var _a;
          return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium text-gray-900 dark:text-gray-100", children: protocol.label }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-gray-500 dark:text-gray-400", children: protocol.description })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                "input",
                {
                  type: "checkbox",
                  className: "sr-only peer",
                  checked: ((_a = formData.protocols) == null ? void 0 : _a[protocol.key]) ?? false,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    protocols: {
                      ...prev.protocols,
                      [protocol.key]: e.target.checked
                    }
                  }))
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" })
            ] })
          ] }, protocol.key);
        }) }) }) }) }),
        activeSection === "latency" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Latency Configuration", subtitle: "Control response timing and delays", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-6", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Base Latency (ms)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "number",
                  placeholder: "0",
                  value: formData.latency.base_ms,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    latency: { ...prev.latency, base_ms: parseInt(e.target.value) || 0 }
                  }))
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Minimum response time for all requests" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Jitter (ms)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "number",
                  placeholder: "0",
                  value: formData.latency.jitter_ms,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    latency: { ...prev.latency, jitter_ms: parseInt(e.target.value) || 0 }
                  }))
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Random delay variation (± jitter)" })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => handleReset("latency"), children: "Reset" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: () => handleSave("latency"), children: "Save Changes" })
          ] })
        ] }) }) }),
        activeSection === "faults" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Fault Injection", subtitle: "Configure error simulation and failure scenarios", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Enable Fault Injection" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Simulate network failures and server errors" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                "input",
                {
                  type: "checkbox",
                  className: "sr-only peer",
                  checked: formData.faults.enabled,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    faults: { ...prev.faults, enabled: e.target.checked }
                  }))
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" })
            ] })
          ] }),
          formData.faults.enabled && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Failure Rate (%)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "number",
                  min: "0",
                  max: "100",
                  placeholder: "5",
                  value: formData.faults.failure_rate,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    faults: { ...prev.faults, failure_rate: parseInt(e.target.value) || 0 }
                  }))
                }
              )
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Error Status Codes" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex flex-wrap gap-2", children: [500, 502, 503, 504, 400, 401, 403, 404].map((code) => /* @__PURE__ */ jsxRuntimeExports.jsx(
                "button",
                {
                  onClick: () => {
                    setFormData((prev) => ({
                      ...prev,
                      faults: {
                        ...prev.faults,
                        status_codes: prev.faults.status_codes.includes(code) ? prev.faults.status_codes.filter((c) => c !== code) : [...prev.faults.status_codes, code]
                      }
                    }));
                  },
                  className: "cursor-pointer",
                  children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                    ModernBadge,
                    {
                      variant: formData.faults.status_codes.includes(code) ? "error" : "outline",
                      children: code
                    }
                  )
                },
                code
              )) })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => handleReset("faults"), children: "Reset" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: () => handleSave("faults"), children: "Save Changes" })
          ] })
        ] }) }) }),
        activeSection === "traffic-shaping" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Traffic Shaping", subtitle: "Control bandwidth and simulate network conditions", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-8", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Enable Traffic Shaping" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Simulate real network conditions with bandwidth control and connectivity issues" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                "input",
                {
                  type: "checkbox",
                  className: "sr-only peer",
                  checked: formData.trafficShaping.enabled,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    trafficShaping: { ...prev.trafficShaping, enabled: e.target.checked }
                  }))
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" })
            ] })
          ] }),
          formData.trafficShaping.enabled && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "border-t border-gray-200 dark:border-gray-700 pt-6", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3 mb-4", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Wifi, { className: "h-5 w-5 text-blue-600" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-medium text-gray-900 dark:text-gray-100", children: "Bandwidth Control" })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between mb-4", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Enable Bandwidth Throttling" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Limit data transfer rates using token bucket algorithm" })
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    "input",
                    {
                      type: "checkbox",
                      className: "sr-only peer",
                      checked: formData.trafficShaping.bandwidth.enabled,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          bandwidth: { ...prev.trafficShaping.bandwidth, enabled: e.target.checked }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" })
                ] })
              ] }),
              formData.trafficShaping.bandwidth.enabled && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-6", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Max Bandwidth (bytes/sec)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    Input$1,
                    {
                      type: "number",
                      min: "1",
                      placeholder: "1048576",
                      value: formData.trafficShaping.bandwidth.max_bytes_per_sec,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          bandwidth: {
                            ...prev.trafficShaping.bandwidth,
                            max_bytes_per_sec: parseInt(e.target.value) || 1048576
                          }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Maximum data transfer rate (1 MB/s = 1,048,576 bytes)" })
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Burst Capacity (bytes)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    Input$1,
                    {
                      type: "number",
                      min: "1",
                      placeholder: "10485760",
                      value: formData.trafficShaping.bandwidth.burst_capacity_bytes,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          bandwidth: {
                            ...prev.trafficShaping.bandwidth,
                            burst_capacity_bytes: parseInt(e.target.value) || 10485760
                          }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Token bucket capacity for burst traffic (10 MB = 10,485,760 bytes)" })
                ] })
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "border-t border-gray-200 dark:border-gray-700 pt-6", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3 mb-4", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(WifiOff, { className: "h-5 w-5 text-orange-600" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-medium text-gray-900 dark:text-gray-100", children: "Burst Loss Simulation" })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between mb-4", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Enable Burst Loss" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Simulate intermittent connectivity issues and packet loss" })
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    "input",
                    {
                      type: "checkbox",
                      className: "sr-only peer",
                      checked: formData.trafficShaping.burstLoss.enabled,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          burstLoss: { ...prev.trafficShaping.burstLoss, enabled: e.target.checked }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" })
                ] })
              ] }),
              formData.trafficShaping.burstLoss.enabled && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-6", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Burst Probability (%)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    Input$1,
                    {
                      type: "number",
                      min: "0",
                      max: "100",
                      step: "0.1",
                      placeholder: "10",
                      value: formData.trafficShaping.burstLoss.burst_probability * 100,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          burstLoss: {
                            ...prev.trafficShaping.burstLoss,
                            burst_probability: parseFloat(e.target.value) / 100 || 0.1
                          }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Probability of entering a loss burst (0-100%)" })
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Burst Duration (ms)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    Input$1,
                    {
                      type: "number",
                      min: "100",
                      placeholder: "5000",
                      value: formData.trafficShaping.burstLoss.burst_duration_ms,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          burstLoss: {
                            ...prev.trafficShaping.burstLoss,
                            burst_duration_ms: parseInt(e.target.value) || 5e3
                          }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Duration of loss bursts in milliseconds" })
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Loss Rate During Burst (%)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    Input$1,
                    {
                      type: "number",
                      min: "0",
                      max: "100",
                      step: "0.1",
                      placeholder: "50",
                      value: formData.trafficShaping.burstLoss.loss_rate_during_burst * 100,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          burstLoss: {
                            ...prev.trafficShaping.burstLoss,
                            loss_rate_during_burst: parseFloat(e.target.value) / 100 || 0.5
                          }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Packet loss rate during burst periods (0-100%)" })
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Recovery Time (ms)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx(
                    Input$1,
                    {
                      type: "number",
                      min: "1000",
                      placeholder: "30000",
                      value: formData.trafficShaping.burstLoss.recovery_time_ms,
                      onChange: (e) => setFormData((prev) => ({
                        ...prev,
                        trafficShaping: {
                          ...prev.trafficShaping,
                          burstLoss: {
                            ...prev.trafficShaping.burstLoss,
                            recovery_time_ms: parseInt(e.target.value) || 3e4
                          }
                        }
                      }))
                    }
                  ),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 dark:text-gray-400 mt-1", children: "Recovery period between bursts in milliseconds" })
                ] })
              ] })
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => handleReset("traffic-shaping"), children: "Reset" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: () => handleSave("traffic-shaping"), children: "Save Changes" })
          ] })
        ] }) }) }),
        activeSection === "proxy" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Proxy Configuration", subtitle: "Configure upstream proxy settings", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Enable Proxy Mode" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Forward requests to upstream services" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("label", { className: "relative inline-flex items-center cursor-pointer", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                "input",
                {
                  type: "checkbox",
                  className: "sr-only peer",
                  checked: formData.proxy.enabled,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    proxy: { ...prev.proxy, enabled: e.target.checked }
                  }))
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600" })
            ] })
          ] }),
          formData.proxy.enabled && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Upstream URL" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "url",
                  placeholder: "https://api.example.com",
                  value: formData.proxy.upstream_url,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    proxy: { ...prev.proxy, upstream_url: e.target.value }
                  })),
                  className: formData.proxy.upstream_url && !isValidUrl(formData.proxy.upstream_url) ? "border-red-500 dark:border-red-500" : ""
                }
              ),
              formData.proxy.upstream_url && !isValidUrl(formData.proxy.upstream_url) && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-red-600 dark:text-red-400 mt-1", children: "Must be a valid HTTP or HTTPS URL" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Timeout (seconds)" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "number",
                  min: "1",
                  max: "300",
                  placeholder: "30",
                  value: formData.proxy.timeout_seconds,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    proxy: { ...prev.proxy, timeout_seconds: parseInt(e.target.value) || 30 }
                  }))
                }
              )
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => handleReset("proxy"), children: "Reset" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: () => handleSave("proxy"), children: "Save Changes" })
          ] })
        ] }) }) }),
        activeSection === "validation" && /* @__PURE__ */ jsxRuntimeExports.jsx(Section, { title: "Validation Settings", subtitle: "Configure request and response validation", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ModernCard, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Validation Mode" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs(
              "select",
              {
                value: formData.validation.mode,
                onChange: (e) => setFormData((prev) => ({
                  ...prev,
                  validation: { ...prev.validation, mode: e.target.value }
                })),
                className: "w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100",
                children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "enforce", children: "Enforce (Strict)" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "warn", children: "Warn Only" }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "off", children: "Disabled" })
                ]
              }
            )
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Aggregate Errors" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Collect all validation errors before responding" })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                "input",
                {
                  type: "checkbox",
                  checked: formData.validation.aggregate_errors,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    validation: { ...prev.validation, aggregate_errors: e.target.checked }
                  })),
                  className: "w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                }
              )
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-sm font-medium text-gray-900 dark:text-gray-100", children: "Validate Responses" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: "Check response format and content" })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                "input",
                {
                  type: "checkbox",
                  checked: formData.validation.validate_responses,
                  onChange: (e) => setFormData((prev) => ({
                    ...prev,
                    validation: { ...prev.validation, validate_responses: e.target.checked }
                  })),
                  className: "w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                }
              )
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => handleReset("validation"), children: "Reset" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: () => handleSave("validation"), children: "Save Changes" })
          ] })
        ] }) }) }),
        activeSection === "environment" && /* @__PURE__ */ jsxRuntimeExports.jsxs(Section, { title: "Environments & Variables", subtitle: "Manage environments and their variables", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            EnvironmentManager,
            {
              workspaceId,
              onEnvironmentSelect: (_envId) => {
              }
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-8", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(ModernCard, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-4", children: "Template Testing" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-sm text-gray-600 dark:text-gray-400 mb-4", children: [
              "Test variable substitution in templates. Type ",
              "{{",
              " to see available variables."
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Template Input (with autocomplete)" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  AutocompleteInput,
                  {
                    value: formData.templateTest || "",
                    onChange: (value) => setFormData((prev) => ({ ...prev, templateTest: value })),
                    placeholder: "Type {{ to see available variables...",
                    workspaceId,
                    context: "template_test",
                    className: "w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                  }
                )
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Expected Output" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-3 bg-gray-50 dark:bg-gray-800 rounded-lg font-mono text-sm text-gray-600 dark:text-gray-400", children: formData.templateTest || "Template output will appear here..." })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xs text-gray-500 dark:text-gray-400", children: "💡 Tip: Use Ctrl+Space anywhere in a text input to manually trigger autocomplete" })
            ] })
          ] }) })
        ] })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open: showRestartDialog, onOpenChange: setShowRestartDialog, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Restart Server Required" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogClose, { onClick: handleCancelRestart })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { children: "Port configuration changes require a server restart to take effect." }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "py-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2 text-sm", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium", children: "HTTP Port:" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: formData.general.http_port })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium", children: "WebSocket Port:" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: formData.general.ws_port })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium", children: "gRPC Port:" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: formData.general.grpc_port })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-medium", children: "Admin Port:" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: formData.general.admin_port })
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: handleCancelRestart, children: "Cancel" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: handleConfirmRestart, children: "Restart Server" })
      ] })
    ] }) })
  ] });
}
const API_BASE$4 = "/api/v1";
async function fetchBYOKConfig() {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$4}/settings/byok`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    if (response.status === 404) {
      return {
        provider: "openai",
        api_key: "",
        enabled: false
      };
    }
    throw new Error("Failed to fetch BYOK config");
  }
  return response.json();
}
async function saveBYOKConfig(config) {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$4}/settings/byok`, {
    method: "PUT",
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(config)
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || "Failed to save BYOK config");
  }
}
const PROVIDERS = [
  {
    value: "openai",
    label: "OpenAI",
    description: "Use OpenAI API (GPT-4, GPT-3.5, etc.)",
    baseUrl: "https://api.openai.com/v1",
    docsUrl: "https://platform.openai.com/docs"
  },
  {
    value: "anthropic",
    label: "Anthropic",
    description: "Use Anthropic API (Claude)",
    baseUrl: "https://api.anthropic.com/v1",
    docsUrl: "https://docs.anthropic.com"
  },
  {
    value: "together",
    label: "Together AI",
    description: "Use Together AI for open-source models",
    baseUrl: "https://api.together.xyz/v1",
    docsUrl: "https://docs.together.ai"
  },
  {
    value: "fireworks",
    label: "Fireworks AI",
    description: "Use Fireworks AI for fast inference",
    baseUrl: "https://api.fireworks.ai/inference/v1",
    docsUrl: "https://docs.fireworks.ai"
  },
  {
    value: "custom",
    label: "Custom",
    description: "Use a custom OpenAI-compatible API",
    baseUrl: "",
    docsUrl: ""
  }
];
function BYOKConfigPage() {
  const { showToast } = useToast();
  const queryClient2 = useQueryClient();
  const [showApiKey, setShowApiKey] = reactExports.useState(false);
  const [config, setConfig] = reactExports.useState({
    provider: "openai",
    api_key: "",
    enabled: false
  });
  const { data: savedConfig, isLoading } = useQuery({
    queryKey: ["byok-config"],
    queryFn: fetchBYOKConfig,
    onSuccess: (data) => {
      setConfig(data);
    }
  });
  const saveMutation = useMutation({
    mutationFn: saveBYOKConfig,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["byok-config"] });
      showToast({
        title: "Success",
        description: "BYOK configuration saved successfully"
      });
    },
    onError: (error) => {
      showToast({
        title: "Error",
        description: error.message || "Failed to save configuration",
        variant: "destructive"
      });
    }
  });
  const handleSave = () => {
    if (!config.api_key.trim() && config.enabled) {
      showToast({
        title: "Error",
        description: "API key is required when BYOK is enabled",
        variant: "destructive"
      });
      return;
    }
    saveMutation.mutate(config);
  };
  const selectedProvider = PROVIDERS.find((p) => p.value === config.provider);
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "container mx-auto p-6 space-y-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold", children: "Bring Your Own Key (BYOK)" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mt-2", children: "Configure your own AI provider API keys for Free tier or additional capacity" })
    ] }),
    isLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Loading configuration..." }) : /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid gap-6 md:grid-cols-3", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: "md:col-span-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Key, { className: "w-5 h-5 mr-2" }),
            "Configuration"
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Set up your AI provider API key to use your own credits" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { children: "AI Provider" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "grid grid-cols-2 gap-3 mt-2", children: PROVIDERS.map((provider) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
              "div",
              {
                className: `p-4 border rounded-lg cursor-pointer transition-colors ${config.provider === provider.value ? "border-primary bg-primary/5" : "hover:bg-accent"}`,
                onClick: () => setConfig({
                  ...config,
                  provider: provider.value,
                  base_url: provider.baseUrl || config.base_url
                }),
                children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: provider.label }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground mt-1", children: provider.description })
                ]
              },
              provider.value
            )) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { htmlFor: "api-key", children: "API Key" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative mt-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  id: "api-key",
                  type: showApiKey ? "text" : "password",
                  placeholder: "sk-...",
                  value: config.api_key,
                  onChange: (e) => setConfig({ ...config, api_key: e.target.value }),
                  className: "pr-10"
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Button$1,
                {
                  variant: "ghost",
                  size: "sm",
                  className: "absolute right-2 top-1/2 -translate-y-1/2",
                  onClick: () => setShowApiKey(!showApiKey),
                  children: showApiKey ? /* @__PURE__ */ jsxRuntimeExports.jsx(EyeOff, { className: "w-4 h-4" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Eye, { className: "w-4 h-4" })
                }
              )
            ] }),
            (selectedProvider == null ? void 0 : selectedProvider.docsUrl) && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-1", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(
              "a",
              {
                href: selectedProvider.docsUrl,
                target: "_blank",
                rel: "noopener noreferrer",
                className: "text-sm text-primary hover:underline flex items-center",
                children: [
                  "View API documentation",
                  /* @__PURE__ */ jsxRuntimeExports.jsx(ExternalLink, { className: "w-3 h-3 ml-1" })
                ]
              }
            ) })
          ] }),
          config.provider === "custom" && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { htmlFor: "base-url", children: "Base URL" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              Input$1,
              {
                id: "base-url",
                type: "url",
                placeholder: "https://api.example.com/v1",
                value: config.base_url || "",
                onChange: (e) => setConfig({ ...config, base_url: e.target.value }),
                className: "mt-2"
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-muted-foreground mt-1", children: "Base URL for your OpenAI-compatible API" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between p-4 border rounded-lg", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: "Enable BYOK" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: "Use your own API key for AI features" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              Button$1,
              {
                variant: config.enabled ? "default" : "outline",
                onClick: () => setConfig({ ...config, enabled: !config.enabled }),
                children: config.enabled ? /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2" }),
                  "Enabled"
                ] }) : /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(CircleX, { className: "w-4 h-4 mr-2" }),
                  "Disabled"
                ] })
              }
            )
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              onClick: handleSave,
              disabled: saveMutation.isPending,
              className: "w-full",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Save, { className: "w-4 h-4 mr-2" }),
                saveMutation.isPending ? "Saving..." : "Save Configuration"
              ]
            }
          )
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(CardHeader, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Info, { className: "w-5 h-5 mr-2" }),
          "About BYOK"
        ] }) }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-semibold mb-2", children: "Free Tier" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-muted-foreground", children: "On the Free plan, BYOK is required to use AI features. Connect your own API key to get started." })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-semibold mb-2", children: "Paid Plans" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-muted-foreground", children: "Pro and Team plans include hosted AI credits, but you can still use BYOK for additional capacity or custom models." })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-semibold mb-2", children: "Security" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-muted-foreground", children: "Your API keys are encrypted and stored securely. They are only used for AI requests you make." })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-3", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(CircleAlert, { className: "w-4 h-4 mr-2 text-yellow-600 dark:text-yellow-400 mt-0.5" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-yellow-800 dark:text-yellow-200", children: "Keep your API keys secure. Never share them publicly or commit them to version control." })
          ] }) })
        ] })
      ] })
    ] })
  ] });
}
function StatusIcon({ status }) {
  switch (status) {
    case "operational":
      return /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "h-5 w-5 text-green-500" });
    case "degraded":
      return /* @__PURE__ */ jsxRuntimeExports.jsx(TriangleAlert, { className: "h-5 w-5 text-yellow-500" });
    case "down":
      return /* @__PURE__ */ jsxRuntimeExports.jsx(CircleX, { className: "h-5 w-5 text-red-500" });
    default:
      return /* @__PURE__ */ jsxRuntimeExports.jsx(Clock, { className: "h-5 w-5 text-gray-500" });
  }
}
function StatusBadge({ status }) {
  const colors = {
    operational: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
    degraded: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
    down: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "span",
    {
      className: `px-2 py-1 rounded-full text-xs font-medium ${colors[status] || "bg-gray-100 text-gray-800"}`,
      children: status.charAt(0).toUpperCase() + status.slice(1)
    }
  );
}
function StatusPage() {
  const { data: status, isLoading, error } = useQuery({
    queryKey: ["status"],
    queryFn: async () => {
      const response = await fetch("/api/v1/status");
      if (!response.ok) {
        throw new Error("Failed to fetch status");
      }
      return response.json();
    },
    refetchInterval: 6e4
    // Refresh every minute
  });
  if (isLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto px-4 py-8 max-w-4xl", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-center py-12", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "animate-spin rounded-full h-8 w-8 border-b-2 border-primary" }) }) });
  }
  if (error) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto px-4 py-8 max-w-4xl", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Alert, { className: "bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800", children: /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-red-800 dark:text-red-200", children: "Failed to load status information. Please try again later." }) }) });
  }
  if (!status) {
    return null;
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "container mx-auto px-4 py-8 max-w-4xl", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "mb-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold mb-2", children: "Service Status" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground", children: "Real-time status of MockForge Cloud services" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { className: "mb-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(StatusIcon, { status: status.status }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { children: [
            "All Systems ",
            status.status === "operational" ? "Operational" : status.status === "degraded" ? "Degraded" : "Down"
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(StatusBadge, { status: status.status })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(CardDescription, { children: [
        "Last updated: ",
        new Date(status.timestamp).toLocaleString()
      ] })
    ] }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: "mb-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(CardHeader, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Activity, { className: "h-5 w-5" }),
        "Services"
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: status.services.map((service) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "div",
        {
          className: "flex items-center justify-between p-4 border rounded-lg",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(StatusIcon, { status: service.status }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: service.name }),
                service.message && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: service.message })
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(StatusBadge, { status: service.status })
          ]
        },
        service.name
      )) }) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(CardHeader, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { children: "Recent Incidents" }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { children: status.incidents.length === 0 ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center py-8 text-muted-foreground", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "h-12 w-12 mx-auto mb-4 text-green-500" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { children: "No incidents reported. All systems operational." })
      ] }) : /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: status.incidents.map((incident) => /* @__PURE__ */ jsxRuntimeExports.jsx(
        "div",
        {
          className: "p-4 border rounded-lg",
          children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start justify-between mb-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-medium", children: incident.title }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-sm text-muted-foreground", children: [
                "Started: ",
                new Date(incident.started_at).toLocaleString(),
                incident.resolved_at && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
                  " • Resolved: ",
                  new Date(incident.resolved_at).toLocaleString()
                ] })
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(StatusBadge, { status: incident.status }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(StatusBadge, { status: incident.impact })
            ] })
          ] })
        },
        incident.id
      )) }) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-6 text-center text-sm text-muted-foreground", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { children: [
      "Status page updates automatically every minute. For more information, visit",
      " ",
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        "a",
        {
          href: "https://docs.mockforge.dev",
          target: "_blank",
          rel: "noopener noreferrer",
          className: "text-primary hover:underline",
          children: "our documentation"
        }
      ),
      " ",
      "or",
      " ",
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        "a",
        {
          href: "/support",
          className: "text-primary hover:underline",
          children: "contact support"
        }
      ),
      "."
    ] }) })
  ] });
}
const API_BASE$3 = "/api/v1";
async function fetchOrganizations() {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$3}/organizations`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to fetch organizations");
  }
  return response.json();
}
async function fetchOrgMembers(orgId) {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$3}/organizations/${orgId}/members`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to fetch members");
  }
  return response.json();
}
function OrganizationPage() {
  const { showToast } = useToast();
  useQueryClient();
  const [selectedOrgId, setSelectedOrgId] = reactExports.useState(null);
  const { data: organizations, isLoading: orgsLoading } = useQuery({
    queryKey: ["organizations"],
    queryFn: fetchOrganizations
  });
  const { data: members, isLoading: membersLoading } = useQuery({
    queryKey: ["org-members", selectedOrgId],
    queryFn: () => fetchOrgMembers(selectedOrgId),
    enabled: !!selectedOrgId
  });
  const getRoleIcon = (role) => {
    switch (role) {
      case "owner":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Crown, { className: "w-4 h-4 text-yellow-500" });
      case "admin":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Shield, { className: "w-4 h-4 text-blue-500" });
      default:
        return /* @__PURE__ */ jsxRuntimeExports.jsx(User, { className: "w-4 h-4 text-gray-500" });
    }
  };
  const getRoleBadge = (role) => {
    switch (role) {
      case "owner":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { className: "bg-yellow-500", children: "Owner" });
      case "admin":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { className: "bg-blue-500", children: "Admin" });
      default:
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: "secondary", children: "Member" });
    }
  };
  const getPlanBadge = (plan) => {
    switch (plan) {
      case "team":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { className: "bg-purple-500", children: "Team" });
      case "pro":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { className: "bg-blue-500", children: "Pro" });
      default:
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: "secondary", children: "Free" });
    }
  };
  if (orgsLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Loading organizations..." }) });
  }
  const selectedOrg = organizations == null ? void 0 : organizations.find((org) => org.id === selectedOrgId);
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "container mx-auto p-6 space-y-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold", children: "Organizations" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mt-2", children: "Manage your organizations and team members" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid gap-6 md:grid-cols-2", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Building2, { className: "w-5 h-5 mr-2" }),
            "Your Organizations"
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Select an organization to manage" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { children: organizations && organizations.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2", children: organizations.map((org) => /* @__PURE__ */ jsxRuntimeExports.jsx(
          "div",
          {
            className: `p-4 border rounded-lg cursor-pointer transition-colors ${selectedOrgId === org.id ? "border-primary bg-primary/5" : "hover:bg-accent"}`,
            onClick: () => setSelectedOrgId(org.id),
            children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-semibold", children: org.name }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-sm text-muted-foreground", children: [
                  "@",
                  org.slug
                ] })
              ] }),
              getPlanBadge(org.plan)
            ] })
          },
          org.id
        )) }) : /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-8 text-muted-foreground", children: "No organizations found" }) })
      ] }),
      selectedOrg ? /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: selectedOrg.name }),
            getPlanBadge(selectedOrg.plan)
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardDescription, { children: [
            "@",
            selectedOrg.slug
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Tabs, { defaultValue: "members", className: "w-full", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsList, { className: "grid w-full grid-cols-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "members", children: "Members" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "settings", children: "Settings" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "members", className: "space-y-4 mt-4", children: membersLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-4", children: "Loading members..." }) : members && members.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2", children: members.map((member) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
            "div",
            {
              className: "flex items-center justify-between p-3 border rounded-lg",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-3", children: [
                  getRoleIcon(member.role),
                  /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: member.username }),
                    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: member.email })
                  ] })
                ] }),
                getRoleBadge(member.role)
              ]
            },
            member.id
          )) }) : /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-8 text-muted-foreground", children: "No members found" }) }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "settings", className: "space-y-4 mt-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { children: "Organization Name" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(Input$1, { value: selectedOrg.name, disabled: true })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { children: "Slug" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(Input$1, { value: selectedOrg.slug, disabled: true })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { children: "Plan" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-2", children: getPlanBadge(selectedOrg.plan) })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { children: "Created" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground mt-1", children: new Date(selectedOrg.created_at).toLocaleDateString() })
            ] })
          ] }) })
        ] }) })
      ] }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "p-12 text-center", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Building2, { className: "w-12 h-12 mx-auto text-muted-foreground mb-4" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold mb-2", children: "Select an Organization" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground", children: "Choose an organization from the list to view details and manage members" })
      ] }) })
    ] })
  ] });
}
const API_BASE$2 = "/api/v1";
async function fetchSubscription() {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$2}/billing/subscription`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to fetch subscription");
  }
  return response.json();
}
async function createCheckout(request) {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$2}/billing/checkout`, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(request)
  });
  if (!response.ok) {
    throw new Error("Failed to create checkout session");
  }
  return response.json();
}
function BillingPage() {
  const { showToast } = useToast();
  useQueryClient();
  const [selectedPlan, setSelectedPlan] = reactExports.useState(null);
  const { data: subscription, isLoading } = useQuery({
    queryKey: ["subscription"],
    queryFn: fetchSubscription
  });
  const checkoutMutation = useMutation({
    mutationFn: createCheckout,
    onSuccess: (data) => {
      window.location.href = data.checkout_url;
    },
    onError: (error) => {
      showToast({
        title: "Error",
        description: error.message || "Failed to create checkout session",
        variant: "destructive"
      });
    }
  });
  const handleUpgrade = (plan) => {
    setSelectedPlan(plan);
    checkoutMutation.mutate({
      plan,
      success_url: `${window.location.origin}/billing?success=true`,
      cancel_url: `${window.location.origin}/billing?canceled=true`
    });
  };
  const formatBytes2 = (bytes) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };
  const formatNumber2 = (num) => {
    if (num >= 1e6) return `${(num / 1e6).toFixed(1)}M`;
    if (num >= 1e3) return `${(num / 1e3).toFixed(1)}K`;
    return num.toString();
  };
  const getUsagePercentage2 = (used, limit) => {
    if (limit <= 0) return 0;
    return Math.min(used / limit * 100, 100);
  };
  const getStatusBadge = (status) => {
    switch (status) {
      case "active":
        return /* @__PURE__ */ jsxRuntimeExports.jsxs(Badge, { className: "bg-green-500", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-3 h-3 mr-1" }),
          "Active"
        ] });
      case "trialing":
        return /* @__PURE__ */ jsxRuntimeExports.jsxs(Badge, { className: "bg-blue-500", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Calendar, { className: "w-3 h-3 mr-1" }),
          "Trialing"
        ] });
      case "past_due":
        return /* @__PURE__ */ jsxRuntimeExports.jsxs(Badge, { className: "bg-yellow-500", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CircleAlert, { className: "w-3 h-3 mr-1" }),
          "Past Due"
        ] });
      case "canceled":
        return /* @__PURE__ */ jsxRuntimeExports.jsxs(Badge, { className: "bg-gray-500", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CircleX, { className: "w-3 h-3 mr-1" }),
          "Canceled"
        ] });
      default:
        return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { children: status });
    }
  };
  if (isLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Loading subscription..." }) });
  }
  if (!subscription) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Failed to load subscription" }) });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "container mx-auto p-6 space-y-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold", children: "Billing & Subscription" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mt-2", children: "Manage your subscription and view usage" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(Tabs, { defaultValue: "overview", className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsList, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "overview", children: "Overview" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "usage", children: "Usage" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "plans", children: "Plans" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "overview", className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid gap-4 md:grid-cols-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Current Plan" }),
              getStatusBadge(subscription.status)
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Your active subscription details" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-2xl font-bold capitalize", children: subscription.plan }),
              subscription.current_period_end && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-sm text-muted-foreground mt-1", children: [
                "Renews on ",
                new Date(subscription.current_period_end).toLocaleDateString()
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Projects" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: subscription.limits.max_projects === -1 ? "Unlimited" : subscription.limits.max_projects })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Collaborators" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: subscription.limits.max_collaborators })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Environments" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: subscription.limits.max_environments })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Hosted Mocks" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: subscription.limits.hosted_mocks ? "Yes" : "No" })
              ] })
            ] }),
            subscription.plan === "free" && /* @__PURE__ */ jsxRuntimeExports.jsxs(
              Button$1,
              {
                onClick: () => handleUpgrade("pro"),
                className: "w-full",
                disabled: checkoutMutation.isPending,
                children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(CircleArrowUp, { className: "w-4 h-4 mr-2" }),
                  "Upgrade to Pro"
                ]
              }
            )
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { children: "Usage This Month" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Current usage against your plan limits" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-1", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "flex items-center", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(TrendingUp, { className: "w-4 h-4 mr-1" }),
                  "Requests"
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
                  formatNumber2(subscription.usage.requests),
                  " /",
                  " ",
                  formatNumber2(subscription.usage.requests_limit)
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-2", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                "div",
                {
                  className: "bg-primary h-2 rounded-full transition-all",
                  style: {
                    width: `${getUsagePercentage2(
                      subscription.usage.requests,
                      subscription.usage.requests_limit
                    )}%`
                  }
                }
              ) })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-1", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "flex items-center", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(HardDrive, { className: "w-4 h-4 mr-1" }),
                  "Storage"
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
                  formatBytes2(subscription.usage.storage_bytes),
                  " /",
                  " ",
                  formatBytes2(subscription.usage.storage_limit_bytes)
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-2", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                "div",
                {
                  className: "bg-primary h-2 rounded-full transition-all",
                  style: {
                    width: `${getUsagePercentage2(
                      subscription.usage.storage_bytes,
                      subscription.usage.storage_limit_bytes
                    )}%`
                  }
                }
              ) })
            ] }),
            subscription.usage.ai_tokens_limit > 0 && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-1", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "flex items-center", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx(Zap, { className: "w-4 h-4 mr-1" }),
                  "AI Tokens"
                ] }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
                  formatNumber2(subscription.usage.ai_tokens_used),
                  " /",
                  " ",
                  formatNumber2(subscription.usage.ai_tokens_limit)
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-2", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                "div",
                {
                  className: "bg-primary h-2 rounded-full transition-all",
                  style: {
                    width: `${getUsagePercentage2(
                      subscription.usage.ai_tokens_used,
                      subscription.usage.ai_tokens_limit
                    )}%`
                  }
                }
              ) })
            ] })
          ] })
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "usage", className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { children: "Detailed Usage" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "View detailed usage statistics" })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-center mb-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("h3", { className: "font-semibold flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(TrendingUp, { className: "w-4 h-4 mr-2" }),
                "API Requests"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm text-muted-foreground", children: [
                formatNumber2(subscription.usage.requests),
                " /",
                " ",
                formatNumber2(subscription.usage.requests_limit)
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-3", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
              "div",
              {
                className: `h-3 rounded-full transition-all ${getUsagePercentage2(
                  subscription.usage.requests,
                  subscription.usage.requests_limit
                ) > 90 ? "bg-red-500" : getUsagePercentage2(
                  subscription.usage.requests,
                  subscription.usage.requests_limit
                ) > 75 ? "bg-yellow-500" : "bg-green-500"}`,
                style: {
                  width: `${getUsagePercentage2(
                    subscription.usage.requests,
                    subscription.usage.requests_limit
                  )}%`
                }
              }
            ) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-center mb-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("h3", { className: "font-semibold flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(HardDrive, { className: "w-4 h-4 mr-2" }),
                "Storage"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm text-muted-foreground", children: [
                formatBytes2(subscription.usage.storage_bytes),
                " /",
                " ",
                formatBytes2(subscription.usage.storage_limit_bytes)
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-3", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
              "div",
              {
                className: `h-3 rounded-full transition-all ${getUsagePercentage2(
                  subscription.usage.storage_bytes,
                  subscription.usage.storage_limit_bytes
                ) > 90 ? "bg-red-500" : getUsagePercentage2(
                  subscription.usage.storage_bytes,
                  subscription.usage.storage_limit_bytes
                ) > 75 ? "bg-yellow-500" : "bg-green-500"}`,
                style: {
                  width: `${getUsagePercentage2(
                    subscription.usage.storage_bytes,
                    subscription.usage.storage_limit_bytes
                  )}%`
                }
              }
            ) })
          ] }),
          subscription.usage.ai_tokens_limit > 0 && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between items-center mb-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("h3", { className: "font-semibold flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Zap, { className: "w-4 h-4 mr-2" }),
                "AI Tokens"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm text-muted-foreground", children: [
                formatNumber2(subscription.usage.ai_tokens_used),
                " /",
                " ",
                formatNumber2(subscription.usage.ai_tokens_limit)
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-3", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
              "div",
              {
                className: `h-3 rounded-full transition-all ${getUsagePercentage2(
                  subscription.usage.ai_tokens_used,
                  subscription.usage.ai_tokens_limit
                ) > 90 ? "bg-red-500" : getUsagePercentage2(
                  subscription.usage.ai_tokens_used,
                  subscription.usage.ai_tokens_limit
                ) > 75 ? "bg-yellow-500" : "bg-green-500"}`,
                style: {
                  width: `${getUsagePercentage2(
                    subscription.usage.ai_tokens_used,
                    subscription.usage.ai_tokens_limit
                  )}%`
                }
              }
            ) })
          ] })
        ] }) })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "plans", className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid gap-4 md:grid-cols-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: subscription.plan === "free" ? "border-primary" : "", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { children: "Free" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-3xl font-bold", children: "$0" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "per month" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("ul", { className: "space-y-2 text-sm", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "1 Project"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "1 Collaborator"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "10K requests/month"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "1GB storage"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleX, { className: "w-4 h-4 mr-2 text-gray-400" }),
                "BYOK only for AI"
              ] })
            ] }),
            subscription.plan === "free" ? /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { disabled: true, className: "w-full", children: "Current Plan" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", className: "w-full", disabled: true, children: "Downgrade" })
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: subscription.plan === "pro" ? "border-primary" : "", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { children: "Pro" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-3xl font-bold", children: "$19" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "per month" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("ul", { className: "space-y-2 text-sm", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "10 Projects"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "5 Collaborators"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "250K requests/month"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "20GB storage"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "100K AI tokens/month"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "Hosted mocks"
              ] })
            ] }),
            subscription.plan === "pro" ? /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { disabled: true, className: "w-full", children: "Current Plan" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(
              Button$1,
              {
                onClick: () => handleUpgrade("pro"),
                className: "w-full",
                disabled: checkoutMutation.isPending,
                children: checkoutMutation.isPending && selectedPlan === "pro" ? "Processing..." : "Upgrade to Pro"
              }
            )
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: subscription.plan === "team" ? "border-primary" : "", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { children: "Team" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-3xl font-bold", children: "$79" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "per month" })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "space-y-4", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("ul", { className: "space-y-2 text-sm", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "Unlimited Projects"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "20 Collaborators"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "1M requests/month"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "100GB storage"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "1M AI tokens/month"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("li", { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "w-4 h-4 mr-2 text-green-500" }),
                "Hosted mocks"
              ] })
            ] }),
            subscription.plan === "team" ? /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { disabled: true, className: "w-full", children: "Current Plan" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(
              Button$1,
              {
                onClick: () => handleUpgrade("team"),
                className: "w-full",
                disabled: checkoutMutation.isPending,
                children: checkoutMutation.isPending && selectedPlan === "team" ? "Processing..." : "Upgrade to Team"
              }
            )
          ] })
        ] })
      ] }) })
    ] })
  ] });
}
const API_BASE$1 = "/api/v1";
async function fetchTokens() {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$1}/tokens`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to fetch tokens");
  }
  return response.json();
}
async function createToken(request) {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$1}/tokens`, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(request)
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || "Failed to create token");
  }
  return response.json();
}
async function deleteToken(tokenId) {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE$1}/tokens/${tokenId}`, {
    method: "DELETE",
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to delete token");
  }
}
const AVAILABLE_SCOPES = [
  { value: "read:packages", label: "Read Packages", description: "Read and search packages" },
  { value: "publish:packages", label: "Publish Packages", description: "Publish new package versions" },
  { value: "read:projects", label: "Read Projects", description: "Read project information" },
  { value: "write:projects", label: "Write Projects", description: "Create and update projects" },
  { value: "deploy:mocks", label: "Deploy Mocks", description: "Deploy hosted mock services" },
  { value: "admin:org", label: "Admin Organization", description: "Full organization administration" }
];
function ApiTokensPage() {
  const { showToast } = useToast();
  const queryClient2 = useQueryClient();
  const [isCreateDialogOpen, setIsCreateDialogOpen] = reactExports.useState(false);
  const [newTokenName, setNewTokenName] = reactExports.useState("");
  const [selectedScopes, setSelectedScopes] = reactExports.useState([]);
  const [expiresDays, setExpiresDays] = reactExports.useState(void 0);
  const [newToken, setNewToken] = reactExports.useState(null);
  const [showToken, setShowToken] = reactExports.useState(false);
  const { data: tokens, isLoading } = useQuery({
    queryKey: ["api-tokens"],
    queryFn: fetchTokens
  });
  const createTokenMutation = useMutation({
    mutationFn: createToken,
    onSuccess: (data) => {
      setNewToken(data.token);
      queryClient2.invalidateQueries({ queryKey: ["api-tokens"] });
      setIsCreateDialogOpen(false);
      setNewTokenName("");
      setSelectedScopes([]);
      setExpiresDays(void 0);
    },
    onError: (error) => {
      showToast({
        title: "Error",
        description: error.message || "Failed to create token",
        variant: "destructive"
      });
    }
  });
  const deleteTokenMutation = useMutation({
    mutationFn: deleteToken,
    onSuccess: () => {
      queryClient2.invalidateQueries({ queryKey: ["api-tokens"] });
      showToast({
        title: "Success",
        description: "Token deleted successfully"
      });
    },
    onError: (error) => {
      showToast({
        title: "Error",
        description: error.message || "Failed to delete token",
        variant: "destructive"
      });
    }
  });
  const handleCreateToken = () => {
    if (!newTokenName.trim()) {
      showToast({
        title: "Error",
        description: "Token name is required",
        variant: "destructive"
      });
      return;
    }
    if (selectedScopes.length === 0) {
      showToast({
        title: "Error",
        description: "At least one scope is required",
        variant: "destructive"
      });
      return;
    }
    createTokenMutation.mutate({
      name: newTokenName,
      scopes: selectedScopes,
      expires_days: expiresDays
    });
  };
  const handleCopyToken = (token) => {
    navigator.clipboard.writeText(token);
    showToast({
      title: "Copied",
      description: "Token copied to clipboard"
    });
  };
  const toggleScope = (scope) => {
    setSelectedScopes(
      (prev) => prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope]
    );
  };
  const formatDate = (dateString) => {
    if (!dateString) return "Never";
    return new Date(dateString).toLocaleDateString();
  };
  const isExpired = (expiresAt) => {
    if (!expiresAt) return false;
    return new Date(expiresAt) < /* @__PURE__ */ new Date();
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "container mx-auto p-6 space-y-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold", children: "API Tokens" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mt-2", children: "Manage personal access tokens for CLI and API access" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { onClick: () => setIsCreateDialogOpen(true), children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "w-4 h-4 mr-2" }),
        "Create Token"
      ] })
    ] }),
    newToken && /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open: !!newToken, onOpenChange: () => setNewToken(null), children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { className: "sm:max-w-md", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Token Created" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { children: "Copy this token now. You won't be able to see it again!" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              type: showToken ? "text" : "password",
              value: newToken,
              readOnly: true,
              className: "font-mono text-sm"
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Button$1,
            {
              variant: "ghost",
              size: "sm",
              className: "absolute right-2 top-1/2 -translate-y-1/2",
              onClick: () => setShowToken(!showToken),
              children: showToken ? /* @__PURE__ */ jsxRuntimeExports.jsx(EyeOff, { className: "w-4 h-4" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Eye, { className: "w-4 h-4" })
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-3", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(TriangleAlert, { className: "w-4 h-4 mr-2 text-yellow-600 dark:text-yellow-400 mt-0.5" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-yellow-800 dark:text-yellow-200", children: "Make sure to copy this token. It will not be shown again." })
        ] }) })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            variant: "outline",
            onClick: () => {
              setNewToken(null);
              setShowToken(false);
            },
            children: "Close"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { onClick: () => handleCopyToken(newToken), children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Copy, { className: "w-4 h-4 mr-2" }),
          "Copy Token"
        ] })
      ] })
    ] }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(Dialog, { open: isCreateDialogOpen, onOpenChange: setIsCreateDialogOpen, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogContent, { className: "sm:max-w-lg", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogHeader, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogTitle, { children: "Create API Token" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(DialogDescription, { children: "Create a personal access token for CLI and API access" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { htmlFor: "token-name", children: "Token Name" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "token-name",
              placeholder: "e.g., CLI Development",
              value: newTokenName,
              onChange: (e) => setNewTokenName(e.target.value)
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { children: "Scopes" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "mt-2 space-y-2 max-h-64 overflow-y-auto", children: AVAILABLE_SCOPES.map((scope) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
            "div",
            {
              className: "flex items-start space-x-3 p-3 border rounded-lg hover:bg-accent cursor-pointer",
              onClick: () => toggleScope(scope.value),
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  "input",
                  {
                    type: "checkbox",
                    checked: selectedScopes.includes(scope.value),
                    onChange: () => toggleScope(scope.value),
                    className: "mt-1"
                  }
                ),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex-1", children: [
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "font-medium", children: scope.label }),
                  /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: scope.description })
                ] })
              ]
            },
            scope.value
          )) })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Label, { htmlFor: "expires-days", children: "Expires In (Days)" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              id: "expires-days",
              type: "number",
              placeholder: "Leave empty for no expiration",
              value: expiresDays || "",
              onChange: (e) => setExpiresDays(e.target.value ? parseInt(e.target.value) : void 0),
              min: "1"
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-muted-foreground mt-1", children: "Optional: Set expiration in days. Leave empty for no expiration." })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(DialogFooter, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { variant: "outline", onClick: () => setIsCreateDialogOpen(false), children: "Cancel" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            onClick: handleCreateToken,
            disabled: createTokenMutation.isPending || !newTokenName.trim() || selectedScopes.length === 0,
            children: createTokenMutation.isPending ? "Creating..." : "Create Token"
          }
        )
      ] })
    ] }) }),
    isLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Loading tokens..." }) : tokens && tokens.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: tokens.map((token) => /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex-1", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-2 mb-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-semibold", children: token.name }),
          isExpired(token.expires_at) && /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: "destructive", children: "Expired" }),
          token.expires_at && !isExpired(token.expires_at) && /* @__PURE__ */ jsxRuntimeExports.jsxs(Badge, { variant: "secondary", children: [
            "Expires ",
            formatDate(token.expires_at)
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-4 text-sm text-muted-foreground mb-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Key, { className: "w-4 h-4 mr-1" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "font-mono", children: [
              token.token_prefix,
              "..."
            ] })
          ] }),
          token.last_used_at && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Calendar, { className: "w-4 h-4 mr-1" }),
            "Last used: ",
            formatDate(token.last_used_at)
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex flex-wrap gap-2", children: token.scopes.map((scope) => {
          const scopeInfo = AVAILABLE_SCOPES.find((s) => s.value === scope);
          return /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: "outline", children: (scopeInfo == null ? void 0 : scopeInfo.label) || scope }, scope);
        }) })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(
        Button$1,
        {
          variant: "ghost",
          size: "sm",
          onClick: () => {
            if (confirm("Are you sure you want to delete this token?")) {
              deleteTokenMutation.mutate(token.id);
            }
          },
          disabled: deleteTokenMutation.isPending,
          children: /* @__PURE__ */ jsxRuntimeExports.jsx(Trash2, { className: "w-4 h-4 text-destructive" })
        }
      )
    ] }) }) }, token.id)) }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "p-12 text-center", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Key, { className: "w-12 h-12 mx-auto text-muted-foreground mb-4" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold mb-2", children: "No API Tokens" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mb-4", children: "Create your first API token to get started with CLI and API access" }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { onClick: () => setIsCreateDialogOpen(true), children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "w-4 h-4 mr-2" }),
        "Create Token"
      ] })
    ] }) })
  ] });
}
const API_BASE = "/api/v1";
async function fetchUsage() {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE}/usage`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to fetch usage");
  }
  return response.json();
}
async function fetchUsageHistory() {
  const token = localStorage.getItem("auth_token");
  const response = await fetch(`${API_BASE}/usage/history`, {
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json"
    }
  });
  if (!response.ok) {
    throw new Error("Failed to fetch usage history");
  }
  return response.json();
}
const formatBytes = (bytes) => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};
const formatNumber = (num) => {
  if (num >= 1e6) return `${(num / 1e6).toFixed(1)}M`;
  if (num >= 1e3) return `${(num / 1e3).toFixed(1)}K`;
  return num.toString();
};
const getUsagePercentage = (used, limit) => {
  if (limit <= 0) return 0;
  return Math.min(used / limit * 100, 100);
};
const getUsageColor = (percentage) => {
  if (percentage > 90) return "bg-red-500";
  if (percentage > 75) return "bg-yellow-500";
  return "bg-green-500";
};
function UsageDashboardPage() {
  const { data: usage, isLoading: usageLoading } = useQuery({
    queryKey: ["usage"],
    queryFn: fetchUsage
  });
  const { data: history, isLoading: historyLoading } = useQuery({
    queryKey: ["usage-history"],
    queryFn: fetchUsageHistory
  });
  if (usageLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Loading usage data..." }) });
  }
  if (!usage) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto p-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Failed to load usage data" }) });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "container mx-auto p-6 space-y-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("h1", { className: "text-3xl font-bold", children: "Usage Dashboard" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground mt-2", children: "Monitor your organization's usage and limits" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "p-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Calendar, { className: "w-4 h-4 text-muted-foreground" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm text-muted-foreground", children: [
          "Current Period: ",
          new Date(usage.period_start).toLocaleDateString(),
          " -",
          " ",
          new Date(usage.period_end).toLocaleDateString()
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(Badge, { className: "capitalize", children: [
        usage.plan,
        " Plan"
      ] })
    ] }) }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(Tabs, { defaultValue: "current", className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsList, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "current", children: "Current Usage" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(TabsTrigger, { value: "history", children: "History" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsContent, { value: "current", className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid gap-4 md:grid-cols-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(TrendingUp, { className: "w-5 h-5 mr-2" }),
                "API Requests"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Monthly request usage" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Used" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "font-semibold", children: [
                  formatNumber(usage.usage.requests.used),
                  " /",
                  " ",
                  formatNumber(usage.usage.requests.limit)
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-3", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                "div",
                {
                  className: `h-3 rounded-full transition-all ${getUsageColor(
                    getUsagePercentage(usage.usage.requests.used, usage.usage.requests.limit)
                  )}`,
                  style: {
                    width: `${getUsagePercentage(
                      usage.usage.requests.used,
                      usage.usage.requests.limit
                    )}%`
                  }
                }
              ) }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-xs text-muted-foreground mt-1", children: [
                (usage.usage.requests.limit - usage.usage.requests.used).toLocaleString(),
                " ",
                "requests remaining"
              ] })
            ] }) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(HardDrive, { className: "w-5 h-5 mr-2" }),
                "Storage"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Storage usage" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Used" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "font-semibold", children: [
                  formatBytes(usage.usage.storage.used),
                  " /",
                  " ",
                  formatBytes(usage.usage.storage.limit)
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-3", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                "div",
                {
                  className: `h-3 rounded-full transition-all ${getUsageColor(
                    getUsagePercentage(usage.usage.storage.used, usage.usage.storage.limit)
                  )}`,
                  style: {
                    width: `${getUsagePercentage(
                      usage.usage.storage.used,
                      usage.usage.storage.limit
                    )}%`
                  }
                }
              ) }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-xs text-muted-foreground mt-1", children: [
                formatBytes(usage.usage.storage.limit - usage.usage.storage.used),
                " remaining"
              ] })
            ] }) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Activity, { className: "w-5 h-5 mr-2" }),
                "Data Egress"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "Data transfer usage" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Used" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "font-semibold", children: formatBytes(usage.usage.egress.used) })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xs text-muted-foreground mt-1", children: usage.usage.egress.limit === -1 ? "Unlimited" : `${formatBytes(usage.usage.egress.limit - usage.usage.egress.used)} remaining` })
            ] }) })
          ] }),
          usage.usage.ai_tokens.limit > 0 && /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs(CardTitle, { className: "flex items-center", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Zap, { className: "w-5 h-5 mr-2" }),
                "AI Tokens"
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(CardDescription, { children: "AI token usage" })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-between text-sm mb-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: "Used" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "font-semibold", children: [
                  formatNumber(usage.usage.ai_tokens.used),
                  " /",
                  " ",
                  formatNumber(usage.usage.ai_tokens.limit)
                ] })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "w-full bg-secondary rounded-full h-3", children: /* @__PURE__ */ jsxRuntimeExports.jsx(
                "div",
                {
                  className: `h-3 rounded-full transition-all ${getUsageColor(
                    getUsagePercentage(
                      usage.usage.ai_tokens.used,
                      usage.usage.ai_tokens.limit
                    )
                  )}`,
                  style: {
                    width: `${getUsagePercentage(
                      usage.usage.ai_tokens.used,
                      usage.usage.ai_tokens.limit
                    )}%`
                  }
                }
              ) }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-xs text-muted-foreground mt-1", children: [
                (usage.usage.ai_tokens.limit - usage.usage.ai_tokens.used).toLocaleString(),
                " ",
                "tokens remaining"
              ] })
            ] }) })
          ] })
        ] }),
        Object.values(usage.usage).some(
          (metric) => metric.limit > 0 && getUsagePercentage(metric.used, metric.limit) > 75
        ) && /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { className: "border-yellow-500", children: /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { className: "p-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start space-x-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CircleAlert, { className: "w-5 h-5 text-yellow-500 mt-0.5" }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "font-semibold mb-1", children: "Usage Warning" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-muted-foreground", children: "You're approaching your plan limits. Consider upgrading to avoid service interruptions." })
          ] })
        ] }) }) })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "history", className: "space-y-4", children: historyLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-12", children: "Loading history..." }) : history && history.history.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: history.history.map((period, index) => /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(CardHeader, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(CardTitle, { className: "text-lg", children: new Date(period.period_start).toLocaleDateString("en-US", {
            month: "long",
            year: "numeric"
          }) }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(CardDescription, { children: [
            new Date(period.period_start).toLocaleDateString(),
            " -",
            " ",
            new Date(period.period_end).toLocaleDateString()
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(CardContent, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-2 md:grid-cols-4 gap-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: "Requests" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-lg font-semibold", children: formatNumber(period.requests) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: "Storage" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-lg font-semibold", children: formatBytes(period.storage_bytes) })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: "Egress" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-lg font-semibold", children: formatBytes(period.egress_bytes) })
          ] }),
          usage.usage.ai_tokens.limit > 0 && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm text-muted-foreground", children: "AI Tokens" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-lg font-semibold", children: formatNumber(period.ai_tokens_used) })
          ] })
        ] }) })
      ] }, index)) }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs(CardContent, { className: "p-12 text-center", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Calendar, { className: "w-12 h-12 mx-auto text-muted-foreground mb-4" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold mb-2", children: "No History Available" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-muted-foreground", children: "Usage history will appear here as you use the service" })
      ] }) }) })
    ] })
  ] });
}
function TimeTravelPage() {
  const { data: status, isLoading: statusLoading } = useTimeTravelStatus();
  const { data: cronData, isLoading: cronLoading } = useCronJobs();
  const { data: mutationData, isLoading: mutationLoading } = useMutationRules();
  const enableMutation = useEnableTimeTravel();
  const disableMutation = useDisableTimeTravel();
  const advanceMutation = useAdvanceTime();
  const scaleMutation = useSetTimeScale();
  const resetMutation = useResetTimeTravel();
  const [advanceDuration, setAdvanceDuration] = reactExports.useState("1h");
  const [timeScale, setTimeScale] = reactExports.useState("1.0");
  const [initialTime, setInitialTime] = reactExports.useState("");
  const formatTime = (timeStr) => {
    if (!timeStr) return "Real Time";
    try {
      const date = new Date(timeStr);
      return date.toLocaleString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit"
      });
    } catch {
      return timeStr;
    }
  };
  const handleEnable = () => {
    enableMutation.mutate({
      time: initialTime || void 0,
      scale: timeScale ? parseFloat(timeScale) : void 0
    });
  };
  const handleAdvance = () => {
    if (advanceDuration) {
      advanceMutation.mutate(advanceDuration);
    }
  };
  const handleSetScale = () => {
    const scale = parseFloat(timeScale);
    if (!isNaN(scale) && scale > 0) {
      scaleMutation.mutate(scale);
    }
  };
  if (statusLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "content-width space-y-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(PageHeader, { title: "Time Travel", subtitle: "Temporal simulation controls" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-center py-12", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "mt-4 text-gray-600 dark:text-gray-400", children: "Loading..." })
      ] }) })
    ] });
  }
  const isEnabled = (status == null ? void 0 : status.enabled) ?? false;
  const virtualTime = status == null ? void 0 : status.current_time;
  const scaleFactor = (status == null ? void 0 : status.scale_factor) ?? 1;
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "content-width space-y-8", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(
      PageHeader,
      {
        title: "Time Travel",
        subtitle: "Control virtual time for testing time-dependent applications",
        className: "space-section"
      }
    ),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: "p-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-start justify-between mb-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "div",
            {
              className: cn(
                "p-3 rounded-xl transition-all duration-200",
                isEnabled ? "bg-brand-100 text-brand-600 dark:bg-brand-900/30 dark:text-brand-400" : "bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400"
              ),
              children: /* @__PURE__ */ jsxRuntimeExports.jsx(Clock, { className: "h-6 w-6" })
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-xl font-semibold text-gray-900 dark:text-gray-100", children: "Time Travel Status" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: isEnabled ? "Virtual time is active" : "Using real time" })
          ] })
        ] }),
        isEnabled && /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: "success", className: "animate-fade-in", children: "Active" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-3 gap-4 mb-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1", children: isEnabled ? "Virtual Time" : "Real Time" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100 tabular-nums", children: formatTime(virtualTime || (status == null ? void 0 : status.real_time)) })
        ] }),
        isEnabled && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1", children: "Time Scale" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-2xl font-bold text-brand-600 dark:text-brand-400", children: [
              scaleFactor.toFixed(1),
              "x"
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide mb-1", children: "Real Time" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100 tabular-nums", children: formatTime(status == null ? void 0 : status.real_time) })
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: !isEnabled ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Initial Time (ISO 8601, optional)" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              Input$1,
              {
                type: "text",
                placeholder: "2025-01-01T00:00:00Z",
                value: initialTime,
                onChange: (e) => setInitialTime(e.target.value),
                className: "w-full"
              }
            )
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Time Scale (1.0 = real time)" }),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              Input$1,
              {
                type: "number",
                step: "0.1",
                min: "0.1",
                placeholder: "1.0",
                value: timeScale,
                onChange: (e) => setTimeScale(e.target.value),
                className: "w-full"
              }
            )
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(
          Button$1,
          {
            onClick: handleEnable,
            disabled: enableMutation.isPending,
            className: "w-full bg-brand-600 hover:bg-brand-700 text-white",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(Play, { className: "h-4 w-4 mr-2" }),
              "Enable Time Travel"
            ]
          }
        )
      ] }) : /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "grid grid-cols-1 md:grid-cols-2 gap-4", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: 'Advance Duration (e.g., "1h", "+1 week", "2d")' }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "text",
                  placeholder: "1h",
                  value: advanceDuration,
                  onChange: (e) => setAdvanceDuration(e.target.value),
                  className: "flex-1"
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsxs(
                Button$1,
                {
                  onClick: handleAdvance,
                  disabled: advanceMutation.isPending || !advanceDuration,
                  className: "bg-brand-600 hover:bg-brand-700 text-white",
                  children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsx(FastForward, { className: "h-4 w-4 mr-2" }),
                    "Advance"
                  ]
                }
              )
            ] })
          ] }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2", children: "Time Scale" }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  type: "number",
                  step: "0.1",
                  min: "0.1",
                  placeholder: "1.0",
                  value: timeScale,
                  onChange: (e) => setTimeScale(e.target.value),
                  className: "flex-1"
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsxs(
                Button$1,
                {
                  onClick: handleSetScale,
                  disabled: scaleMutation.isPending || !timeScale,
                  className: "bg-brand-600 hover:bg-brand-700 text-white",
                  children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsx(Zap, { className: "h-4 w-4 mr-2" }),
                    "Set Scale"
                  ]
                }
              )
            ] })
          ] })
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-2", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              onClick: () => disableMutation.mutate(),
              disabled: disableMutation.isPending,
              variant: "outline",
              className: "flex-1",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Pause, { className: "h-4 w-4 mr-2" }),
                "Disable"
              ]
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            Button$1,
            {
              onClick: () => resetMutation.mutate(),
              disabled: resetMutation.isPending,
              variant: "outline",
              className: "flex-1",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(RotateCcw, { className: "h-4 w-4 mr-2" }),
                "Reset to Real Time"
              ]
            }
          )
        ] })
      ] }) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs(Tabs, { defaultValue: "cron", className: "space-y-6", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsList, { className: "grid w-full grid-cols-3", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "cron", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Calendar, { className: "h-4 w-4 mr-2" }),
          "Cron Jobs"
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "mutations", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4 mr-2" }),
          "Mutation Rules"
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(TabsTrigger, { value: "scenarios", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "h-4 w-4 mr-2" }),
          "Scenarios"
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "cron", className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: "p-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4", children: "Scheduled Cron Jobs" }),
        cronLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-8", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-brand-600" }) }) : (cronData == null ? void 0 : cronData.jobs) && cronData.jobs.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-3", children: cronData.jobs.map((job) => /* @__PURE__ */ jsxRuntimeExports.jsx(
          "div",
          {
            className: "p-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50",
            children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-semibold text-gray-900 dark:text-gray-100", children: job.name }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: job.schedule }),
                job.description && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500 dark:text-gray-500 mt-1", children: job.description })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: job.enabled ? "success" : "default", children: job.enabled ? "Enabled" : "Disabled" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-xs text-gray-500 dark:text-gray-500", children: [
                  job.execution_count || 0,
                  " executions"
                ] })
              ] })
            ] })
          },
          job.id
        )) }) : /* @__PURE__ */ jsxRuntimeExports.jsx(Alert, { type: "info", title: "No cron jobs", message: "Create cron jobs to schedule recurring events." })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "mutations", className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: "p-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4", children: "Data Mutation Rules" }),
        mutationLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-center py-8", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-brand-600" }) }) : (mutationData == null ? void 0 : mutationData.rules) && mutationData.rules.length > 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-3", children: mutationData.rules.map((rule) => /* @__PURE__ */ jsxRuntimeExports.jsx(
          "div",
          {
            className: "p-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50",
            children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("h4", { className: "font-semibold text-gray-900 dark:text-gray-100", children: rule.id }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-sm text-gray-600 dark:text-gray-400", children: [
                  "Entity: ",
                  rule.entity_name
                ] }),
                rule.description && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500 dark:text-gray-500 mt-1", children: rule.description })
              ] }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Badge, { variant: rule.enabled ? "success" : "default", children: rule.enabled ? "Enabled" : "Disabled" }),
                /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-xs text-gray-500 dark:text-gray-500", children: [
                  rule.execution_count || 0,
                  " executions"
                ] })
              ] })
            ] })
          },
          rule.id
        )) }) : /* @__PURE__ */ jsxRuntimeExports.jsx(
          Alert,
          {
            type: "info",
            title: "No mutation rules",
            message: "Create mutation rules to automatically modify data based on time triggers."
          }
        )
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(TabsContent, { value: "scenarios", className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(Card, { className: "p-6", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4", children: "Time Travel Scenarios" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Alert,
          {
            type: "info",
            title: "Scenario Management",
            message: "Save and load time travel scenarios to quickly restore specific time states. Use the CLI or API to manage scenarios."
          }
        )
      ] }) })
    ] })
  ] });
}
function MobileCard({
  row,
  columns,
  onRowClick,
  showExpandButton = true
}) {
  const [isExpanded, setIsExpanded] = reactExports.useState(false);
  const prioritizedColumns = columns.filter((col) => !col.hideOnMobile).sort((a, b) => {
    const priorityOrder = { high: 0, medium: 1, low: 2 };
    const aPriority = priorityOrder[a.priority || "medium"];
    const bPriority = priorityOrder[b.priority || "medium"];
    return aPriority - bPriority;
  });
  const highPriorityColumns = prioritizedColumns.filter((col) => col.priority === "high");
  const otherColumns = prioritizedColumns.filter((col) => col.priority !== "high");
  return /* @__PURE__ */ jsxRuntimeExports.jsxs(
    "div",
    {
      className: cn(
        "bg-card border border-gray-200 dark:border-gray-800 rounded-lg p-4 space-y-3",
        "table-row-hover spring-in animate-fade-in-up",
        onRowClick && "cursor-pointer"
      ),
      onClick: () => onRowClick == null ? void 0 : onRowClick(row),
      children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2", children: highPriorityColumns.map((column) => {
          const value = row[column.key];
          const displayValue = column.render ? column.render(value, row) : value;
          return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm font-medium text-gray-600 dark:text-gray-400", children: column.mobileLabel || column.label }),
            /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-base text-gray-900 dark:text-gray-100 font-medium", children: displayValue })
          ] }, column.key);
        }) }),
        otherColumns.length > 0 && showExpandButton && /* @__PURE__ */ jsxRuntimeExports.jsxs(jsxRuntimeExports.Fragment, { children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "divider-subtle" }),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            "button",
            {
              className: "flex items-center justify-between w-full text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 transition-colors",
              onClick: (e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              },
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { children: isExpanded ? "Show less" : `Show ${otherColumns.length} more details` }),
                /* @__PURE__ */ jsxRuntimeExports.jsx(ChevronIcon, { direction: isExpanded ? "up" : "down", size: "sm" })
              ]
            }
          ),
          isExpanded && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2 animate-fade-in-up", children: otherColumns.map((column) => {
            const value = row[column.key];
            const displayValue = column.render ? column.render(value, row) : value;
            return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-xs font-medium text-gray-600 dark:text-gray-400", children: column.mobileLabel || column.label }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-sm text-gray-600 dark:text-gray-400", children: displayValue })
            ] }, column.key);
          }) })
        ] })
      ]
    }
  );
}
function DesktopTable({
  columns,
  data,
  onRowClick,
  sortable = false
}) {
  const [sortColumn, setSortColumn] = reactExports.useState(null);
  const [sortDirection, setSortDirection] = reactExports.useState("asc");
  const handleSort = (columnKey) => {
    if (!sortable) return;
    if (sortColumn === columnKey) {
      setSortDirection(sortDirection === "asc" ? "desc" : "asc");
    } else {
      setSortColumn(columnKey);
      setSortDirection("asc");
    }
  };
  const sortedData = React.useMemo(() => {
    if (!sortColumn) return data;
    return [...data].sort((a, b) => {
      const aValue = a[sortColumn];
      const bValue = b[sortColumn];
      if (aValue < bValue) return sortDirection === "asc" ? -1 : 1;
      if (aValue > bValue) return sortDirection === "asc" ? 1 : -1;
      return 0;
    });
  }, [data, sortColumn, sortDirection]);
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "overflow-x-auto custom-scrollbar", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("table", { className: "min-w-full divide-y divide-gray-200 dark:divide-gray-700", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx("thead", { className: "bg-gray-50 dark:bg-gray-800", children: /* @__PURE__ */ jsxRuntimeExports.jsx("tr", { children: columns.map((column) => /* @__PURE__ */ jsxRuntimeExports.jsx(
      "th",
      {
        className: cn(
          "px-6 py-3 text-left text-xs font-medium text-tertiary uppercase tracking-wider",
          column.width && `w-[${column.width}]`,
          column.minWidth && `min-w-[${column.minWidth}]`,
          sortable && column.sortable && "cursor-pointer hover:text-primary transition-colors"
        ),
        style: {
          width: column.width,
          minWidth: column.minWidth
        },
        onClick: () => column.sortable && handleSort(column.key),
        children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
          column.label,
          sortable && column.sortable && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-col", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              ChevronIcon,
              {
                direction: "up",
                size: "xs",
                className: cn(
                  "transition-opacity",
                  sortColumn === column.key && sortDirection === "asc" ? "opacity-100" : "opacity-30"
                )
              }
            ),
            /* @__PURE__ */ jsxRuntimeExports.jsx(
              ChevronIcon,
              {
                direction: "down",
                size: "xs",
                className: cn(
                  "transition-opacity -mt-1",
                  sortColumn === column.key && sortDirection === "desc" ? "opacity-100" : "opacity-30"
                )
              }
            )
          ] })
        ] })
      },
      column.key
    )) }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("tbody", { className: "bg-background divide-y divide-gray-200 dark:divide-gray-700", children: sortedData.map((row, index) => /* @__PURE__ */ jsxRuntimeExports.jsx(
      "tr",
      {
        className: cn(
          "table-row-hover animate-stagger-in",
          onRowClick && "cursor-pointer"
        ),
        style: { animationDelay: `${index * 50}ms` },
        onClick: () => onRowClick == null ? void 0 : onRowClick(row),
        children: columns.map((column) => {
          const value = row[column.key];
          const displayValue = column.render ? column.render(value, row) : value;
          return /* @__PURE__ */ jsxRuntimeExports.jsx(
            "td",
            {
              className: "px-6 py-4 whitespace-nowrap text-base text-gray-900 dark:text-gray-100",
              children: displayValue
            },
            column.key
          );
        })
      },
      index
    )) })
  ] }) });
}
function ResponsiveTable({
  columns,
  data,
  className,
  onRowClick,
  stackOnMobile = true,
  showExpandButton = true,
  sortable = false,
  searchable = false,
  searchPlaceholder = "Search...",
  isLoading = false,
  emptyMessage = "No data available"
}) {
  const [searchQuery, setSearchQuery] = reactExports.useState("");
  const filteredData = React.useMemo(() => {
    if (!searchable || !searchQuery.trim()) return data;
    return data.filter((row) => {
      return columns.some((column) => {
        const value = row[column.key];
        return String(value).toLowerCase().includes(searchQuery.toLowerCase());
      });
    });
  }, [data, searchQuery, columns, searchable]);
  if (isLoading) {
    return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: [...Array(3)].map((_, i) => /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "animate-pulse", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-16 bg-gray-200 dark:bg-gray-700 rounded-lg" }) }, i)) });
  }
  if (filteredData.length === 0) {
    return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center py-12", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-xl font-bold text-gray-600 dark:text-gray-400 mb-2", children: searchQuery ? "No matching results" : emptyMessage }),
      searchQuery && /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-base text-gray-600 dark:text-gray-400", children: "Try adjusting your search terms" })
    ] });
  }
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn("space-y-4", className), children: [
    searchable && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-3", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "relative flex-1 max-w-sm", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "input",
          {
            type: "text",
            placeholder: searchPlaceholder,
            value: searchQuery,
            onChange: (e) => setSearchQuery(e.target.value),
            className: "w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-base bg-background focus:ring-2 focus:ring-brand/20 focus:border-brand transition-colors"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx(Icons.Search, { className: "absolute left-3 top-2.5 h-4 w-4 text-gray-600 dark:text-gray-400" })
      ] }),
      searchQuery && /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button,
        {
          variant: "secondary",
          size: "sm",
          onClick: () => setSearchQuery(""),
          className: "flex items-center gap-2",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Icons.Close, { className: "h-4 w-4" }),
            "Clear"
          ]
        }
      )
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn(
      stackOnMobile ? "md:hidden" : "hidden",
      "space-y-3"
    ), children: filteredData.map((row, index) => /* @__PURE__ */ jsxRuntimeExports.jsx(
      MobileCard,
      {
        row,
        columns,
        onRowClick,
        showExpandButton
      },
      index
    )) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn(
      stackOnMobile ? "hidden md:block" : "block"
    ), children: /* @__PURE__ */ jsxRuntimeExports.jsx(
      DesktopTable,
      {
        columns,
        data: filteredData,
        onRowClick,
        sortable
      }
    ) }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between text-sm text-gray-600 dark:text-gray-400", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
        "Showing ",
        filteredData.length,
        " of ",
        data.length,
        " ",
        data.length === 1 ? "item" : "items"
      ] }),
      searchQuery && /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { children: [
        'Filtered by "',
        searchQuery,
        '"'
      ] })
    ] })
  ] });
}
function Skeleton({
  className,
  width,
  height,
  circle = false,
  animation = "shimmer",
  ...props
}) {
  const animationClasses = {
    pulse: "animate-pulse",
    shimmer: "loading-shimmer",
    none: ""
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(
    "div",
    {
      className: cn(
        "bg-gray-200 dark:bg-gray-700",
        circle ? "rounded-full" : "rounded-md",
        animationClasses[animation],
        className
      ),
      style: {
        width: typeof width === "number" ? `${width}px` : width,
        height: typeof height === "number" ? `${height}px` : height
      },
      ...props
    }
  );
}
function SkeletonCard({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn("p-6 border border-gray-200 dark:border-gray-800 rounded-xl space-y-4", className), ...props, children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center space-x-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { circle: true, width: 48, height: 48 }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2 flex-1", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 16, width: "60%" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, width: "40%" })
      ] })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, width: "100%" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, width: "80%" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, width: "90%" })
    ] })
  ] });
}
function SkeletonTable({ rows = 5, cols = 4, className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn("space-y-4", className), ...props, children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex space-x-4", children: Array.from({ length: cols }).map((_, index) => /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 16, className: "flex-1" }, `header-${index}`)) }),
    Array.from({ length: rows }).map((_, rowIndex) => /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex space-x-4", children: Array.from({ length: cols }).map((_2, colIndex) => /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, className: "flex-1" }, `cell-${rowIndex}-${colIndex}`)) }, `row-${rowIndex}`))
  ] });
}
function SkeletonMetricCard({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: cn("p-6 border border-gray-200 dark:border-gray-800 rounded-xl", className), ...props, children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-2 flex-1", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, width: "60%" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 20, width: "40%" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 10, width: "50%" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { circle: true, width: 40, height: 40 })
  ] }) });
}
function SkeletonChart({ className, ...props }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: cn("p-6 border border-gray-200 dark:border-gray-800 rounded-xl space-y-4", className), ...props, children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 16, width: "30%" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(Skeleton, { height: 12, width: "20%" })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "h-64 flex items-end space-x-2", children: Array.from({ length: 12 }).map((_, index) => /* @__PURE__ */ jsxRuntimeExports.jsx(
      Skeleton,
      {
        className: "flex-1",
        height: Math.random() * 200 + 40
      },
      `bar-${index}`
    )) })
  ] });
}
function ErrorFallback({
  error,
  resetError,
  retry,
  title = "Something went wrong",
  description = "An unexpected error occurred. Please try again.",
  showDetails = false,
  showHomeButton = false,
  className = ""
}) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: `flex flex-col items-center justify-center p-8 text-center ${className}`, children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-4 rounded-full bg-red-50 dark:bg-red-900/20 mb-4", children: /* @__PURE__ */ jsxRuntimeExports.jsx(StatusIcon$1, { status: "error", size: "lg" }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-xl font-bold text-gray-900 dark:text-gray-100 mb-2", children: title }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-base text-gray-600 dark:text-gray-400 mb-6 max-w-md", children: description }),
    showDetails && error && /* @__PURE__ */ jsxRuntimeExports.jsxs("details", { className: "mb-6 w-full max-w-md", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx("summary", { className: "cursor-pointer text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:text-gray-100 mb-2", children: "Show error details" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-left", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-sm font-mono text-red-700 dark:text-red-500 break-all", children: error.message }) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3", children: [
      retry && /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          onClick: retry,
          className: "flex items-center gap-2",
          variant: "default",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4" }),
            "Try again"
          ]
        }
      ),
      resetError && /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          onClick: resetError,
          variant: "outline",
          className: "flex items-center gap-2",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4" }),
            "Reset"
          ]
        }
      ),
      showHomeButton && /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          onClick: () => window.location.href = "/",
          variant: "outline",
          className: "flex items-center gap-2",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(House, { className: "h-4 w-4" }),
            "Go home"
          ]
        }
      )
    ] })
  ] });
}
function DataErrorFallback({ retry, resetError }) {
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-col items-center justify-center p-8 text-center", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-4 rounded-full bg-orange-50 dark:bg-orange-900/20 mb-4", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Database, { className: "h-8 w-8 text-orange-600 dark:text-orange-400" }) }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-xl font-bold text-gray-900 dark:text-gray-100 mb-2", children: "Data Loading Failed" }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-base text-gray-600 dark:text-gray-400 mb-6 max-w-md", children: "There was a problem loading the data. This might be a temporary issue." }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex gap-3", children: [
      retry && /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          onClick: retry,
          className: "flex items-center gap-2",
          variant: "default",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4" }),
            "Reload data"
          ]
        }
      ),
      resetError && /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          onClick: resetError,
          variant: "outline",
          className: "flex items-center gap-2",
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(House, { className: "h-4 w-4" }),
            "Reset view"
          ]
        }
      )
    ] })
  ] });
}
class ErrorBoundaryWrapper extends React.Component {
  constructor(props) {
    super(props);
    this.state = { hasError: false };
  }
  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }
  componentDidCatch(error, errorInfo) {
    logger.error("Component error caught", error, errorInfo);
  }
  render() {
    if (this.state.hasError) {
      const FallbackComponent = this.props.fallback || ErrorFallback;
      return /* @__PURE__ */ jsxRuntimeExports.jsx(
        FallbackComponent,
        {
          error: this.state.error,
          resetError: () => this.setState({ hasError: false, error: void 0 })
        }
      );
    }
    return this.props.children;
  }
}
function useErrorHandling(maxRetries = 3) {
  var _a;
  const [errorState, setErrorState] = reactExports.useState({
    error: null,
    isRetrying: false,
    retryCount: 0
  });
  const queryClient2 = useQueryClient();
  const { error: showErrorToast } = useToastStore();
  const handleError = reactExports.useCallback((error, context) => {
    const showToast = (context == null ? void 0 : context.showToast) ?? true;
    setErrorState((prev) => ({
      ...prev,
      error,
      context: {
        operation: (context == null ? void 0 : context.operation) || "unknown",
        component: context == null ? void 0 : context.component,
        retryable: (context == null ? void 0 : context.retryable) ?? true,
        timestamp: Date.now(),
        showToast
      }
    }));
    logger.error("Error handled", {
      error: error.message,
      stack: error.stack,
      context
    });
    if (showToast) {
      const title = (context == null ? void 0 : context.operation) ? `Failed to ${context.operation}` : "An error occurred";
      showErrorToast(title, error.message);
    }
  }, [showErrorToast]);
  const retry = reactExports.useCallback(async (operation) => {
    if (errorState.retryCount >= maxRetries) {
      logger.warn("Max retries reached, not retrying");
      return;
    }
    setErrorState((prev) => ({
      ...prev,
      isRetrying: true,
      retryCount: prev.retryCount + 1
    }));
    try {
      if (operation) {
        await operation();
      } else {
        await queryClient2.invalidateQueries();
      }
      setErrorState({
        error: null,
        isRetrying: false,
        retryCount: 0
      });
    } catch (error) {
      setErrorState((prev) => ({
        ...prev,
        error: error instanceof Error ? error : new Error("Retry failed"),
        isRetrying: false
      }));
    }
  }, [errorState.retryCount, maxRetries, queryClient2]);
  const clearError = reactExports.useCallback(() => {
    setErrorState({
      error: null,
      isRetrying: false,
      retryCount: 0
    });
  }, []);
  const isNetworkError = reactExports.useCallback((error) => {
    return error.message.includes("fetch") || error.message.includes("network") || error.message.includes("Failed to load");
  }, []);
  const isTimeoutError = reactExports.useCallback((error) => {
    return error.message.includes("timeout") || error.message.includes("Timeout");
  }, []);
  const getErrorType = reactExports.useCallback((error) => {
    if (isNetworkError(error)) return "network";
    if (isTimeoutError(error)) return "timeout";
    return "generic";
  }, [isNetworkError, isTimeoutError]);
  return {
    errorState,
    handleError,
    retry,
    clearError,
    isNetworkError,
    isTimeoutError,
    getErrorType,
    canRetry: errorState.retryCount < maxRetries && ((_a = errorState.context) == null ? void 0 : _a.retryable) !== false
  };
}
function useApiErrorHandling() {
  const { handleError, retry, clearError, errorState, getErrorType, canRetry } = useErrorHandling();
  const handleApiError = reactExports.useCallback((error, operation) => {
    let errorObj;
    if (error instanceof Error) {
      errorObj = error;
    } else if (typeof error === "string") {
      errorObj = new Error(error);
    } else {
      errorObj = new Error("An unknown error occurred");
    }
    handleError(errorObj, {
      operation,
      component: "API",
      retryable: true
    });
  }, [handleError]);
  const retryApiCall = reactExports.useCallback(async (apiCall) => {
    try {
      const result = await apiCall();
      clearError();
      return result;
    } catch (error) {
      handleApiError(error, "retry");
      throw error;
    }
  }, [clearError, handleApiError]);
  return {
    errorState,
    handleApiError,
    retry,
    retryApiCall,
    clearError,
    getErrorType,
    canRetry
  };
}
function ProxyInspector() {
  const [activeTab, setActiveTab] = reactExports.useState("rules");
  const [showCreateForm, setShowCreateForm] = reactExports.useState(false);
  const [editingRule, setEditingRule] = reactExports.useState(null);
  const [filterType, setFilterType] = reactExports.useState("all");
  const [searchPattern, setSearchPattern] = reactExports.useState("");
  const { handleApiError, retry, clearError, errorState, canRetry } = useApiErrorHandling();
  const {
    data: rulesData,
    isLoading: rulesLoading,
    error: rulesError,
    refetch: refetchRules
  } = useProxyRules();
  const {
    data: inspectData,
    isLoading: inspectLoading,
    error: inspectError,
    refetch: refetchInspect
  } = useProxyInspect(50);
  const createRuleMutation = useCreateProxyRule();
  const updateRuleMutation = useUpdateProxyRule();
  const deleteRuleMutation = useDeleteProxyRule();
  React.useEffect(() => {
    if (rulesError) {
      handleApiError(rulesError, "fetch_proxy_rules");
    } else {
      clearError();
    }
  }, [rulesError, handleApiError, clearError]);
  const filteredRules = reactExports.useMemo(() => {
    if (!(rulesData == null ? void 0 : rulesData.rules)) return [];
    let filtered = rulesData.rules;
    if (filterType !== "all") {
      filtered = filtered.filter((rule) => rule.type === filterType);
    }
    if (searchPattern) {
      const searchLower = searchPattern.toLowerCase();
      filtered = filtered.filter(
        (rule) => rule.pattern.toLowerCase().includes(searchLower) || rule.body_transforms.some(
          (t) => t.path.toLowerCase().includes(searchLower) || t.replace.toLowerCase().includes(searchLower)
        )
      );
    }
    return filtered;
  }, [rulesData, filterType, searchPattern]);
  const handleCreateRule = async (formData) => {
    try {
      const ruleRequest = {
        pattern: formData.pattern,
        type: formData.type,
        status_codes: formData.status_codes,
        body_transforms: formData.body_transforms.map((t) => ({
          path: t.path,
          replace: t.replace,
          operation: t.operation || "replace"
        })),
        enabled: formData.enabled
      };
      await createRuleMutation.mutateAsync(ruleRequest);
      setShowCreateForm(false);
    } catch (error) {
      handleApiError(error, "create_proxy_rule");
    }
  };
  const handleUpdateRule = async (id, formData) => {
    try {
      const ruleRequest = {
        pattern: formData.pattern,
        type: formData.type,
        status_codes: formData.status_codes,
        body_transforms: formData.body_transforms.map((t) => ({
          path: t.path,
          replace: t.replace,
          operation: t.operation || "replace"
        })),
        enabled: formData.enabled
      };
      await updateRuleMutation.mutateAsync({ id, rule: ruleRequest });
      setEditingRule(null);
    } catch (error) {
      handleApiError(error, "update_proxy_rule");
    }
  };
  const handleDeleteRule = async (id) => {
    if (!confirm("Are you sure you want to delete this proxy replacement rule?")) {
      return;
    }
    try {
      await deleteRuleMutation.mutateAsync(id);
    } catch (error) {
      handleApiError(error, "delete_proxy_rule");
    }
  };
  const rulesColumns = [
    {
      header: "Pattern",
      accessor: "pattern",
      cell: (rule) => /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center gap-2", children: /* @__PURE__ */ jsxRuntimeExports.jsx("code", { className: "text-xs bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded", children: rule.pattern }) })
    },
    {
      header: "Type",
      accessor: "type",
      cell: (rule) => /* @__PURE__ */ jsxRuntimeExports.jsx(
        Badge$1,
        {
          variant: rule.type === "request" ? "info" : "success",
          className: "text-xs",
          children: rule.type
        }
      )
    },
    {
      header: "Transforms",
      accessor: "body_transforms",
      cell: (rule) => /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex flex-col gap-1", children: rule.body_transforms.map((transform, idx) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-xs text-gray-600 dark:text-gray-400", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("code", { className: "text-xs", children: transform.path }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(ArrowRight, { className: "inline mx-1 h-3 w-3" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-xs", children: [
          transform.replace.substring(0, 30),
          "..."
        ] })
      ] }, idx)) })
    },
    {
      header: "Status",
      accessor: "enabled",
      cell: (rule) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
        rule.enabled ? /* @__PURE__ */ jsxRuntimeExports.jsx(CircleCheck, { className: "h-4 w-4 text-green-600" }) : /* @__PURE__ */ jsxRuntimeExports.jsx(CircleX, { className: "h-4 w-4 text-gray-400" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-xs", children: rule.enabled ? "Enabled" : "Disabled" })
      ] })
    },
    {
      header: "Actions",
      accessor: "id",
      cell: (rule) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            variant: "ghost",
            size: "sm",
            onClick: () => setEditingRule(rule),
            className: "h-8 w-8 p-0",
            children: /* @__PURE__ */ jsxRuntimeExports.jsx(SquarePen, { className: "h-4 w-4" })
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Button$1,
          {
            variant: "ghost",
            size: "sm",
            onClick: () => handleDeleteRule(rule.id),
            className: "h-8 w-8 p-0 text-red-600 hover:text-red-700",
            children: /* @__PURE__ */ jsxRuntimeExports.jsx(Trash2, { className: "h-4 w-4" })
          }
        )
      ] })
    }
  ];
  return /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100", children: "Proxy Inspector" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-600 dark:text-gray-400 mt-1", children: "Inspect and replace requests/responses from browser proxy mode" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center gap-2", children: /* @__PURE__ */ jsxRuntimeExports.jsxs(
        Button$1,
        {
          variant: "outline",
          size: "sm",
          onClick: () => {
            refetchRules();
            refetchInspect();
          },
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-4 w-4 mr-2" }),
            "Refresh"
          ]
        }
      ) })
    ] }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "border-b border-gray-200 dark:border-gray-700", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("nav", { className: "flex space-x-8", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setActiveTab("rules"),
          className: cn(
            "py-4 px-1 border-b-2 font-medium text-sm transition-colors",
            activeTab === "rules" ? "border-brand-500 text-brand-600 dark:text-brand-400" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
          ),
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Settings, { className: "inline h-4 w-4 mr-2" }),
            "Replacement Rules"
          ]
        }
      ),
      /* @__PURE__ */ jsxRuntimeExports.jsxs(
        "button",
        {
          onClick: () => setActiveTab("inspect"),
          className: cn(
            "py-4 px-1 border-b-2 font-medium text-sm transition-colors",
            activeTab === "inspect" ? "border-brand-500 text-brand-600 dark:text-brand-400" : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
          ),
          children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Eye, { className: "inline h-4 w-4 mr-2" }),
            "Intercepted Traffic"
          ]
        }
      )
    ] }) }),
    errorState.hasError && /* @__PURE__ */ jsxRuntimeExports.jsx(
      DataErrorFallback,
      {
        error: errorState.error,
        retry: canRetry ? retry : void 0,
        onDismiss: clearError
      }
    ),
    activeTab === "rules" && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex flex-1 gap-2 items-center", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Funnel, { className: "h-4 w-4 text-gray-400" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            Input$1,
            {
              placeholder: "Search patterns or transforms...",
              value: searchPattern,
              onChange: (e) => setSearchPattern(e.target.value),
              className: "max-w-sm"
            }
          ),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(
            "select",
            {
              value: filterType,
              onChange: (e) => setFilterType(e.target.value),
              className: "px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-sm",
              children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "all", children: "All Types" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "request", children: "Request Rules" }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "response", children: "Response Rules" })
              ]
            }
          )
        ] }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { onClick: () => setShowCreateForm(true), children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "h-4 w-4 mr-2" }),
          "Create Rule"
        ] })
      ] }) }),
      /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: rulesLoading ? /* @__PURE__ */ jsxRuntimeExports.jsx(SkeletonTable, { columns: 5, rows: 5 }) : rulesError ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-8 text-center text-red-600", children: [
        "Failed to load proxy rules. ",
        canRetry && /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: retry, children: "Retry" })
      ] }) : filteredRules.length === 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "p-8 text-center text-gray-500", children: (rulesData == null ? void 0 : rulesData.rules.length) === 0 ? "No proxy replacement rules configured. Create one to get started." : "No rules match your filters." }) : /* @__PURE__ */ jsxRuntimeExports.jsx(
        ResponsiveTable,
        {
          data: filteredRules,
          columns: rulesColumns,
          keyExtractor: (rule) => rule.id.toString()
        }
      ) }),
      (showCreateForm || editingRule) && /* @__PURE__ */ jsxRuntimeExports.jsx(
        ProxyRuleForm,
        {
          rule: editingRule,
          onSave: (formData) => {
            if (editingRule) {
              handleUpdateRule(editingRule.id, formData);
            } else {
              handleCreateRule(formData);
            }
          },
          onCancel: () => {
            setShowCreateForm(false);
            setEditingRule(null);
          }
        }
      )
    ] }),
    activeTab === "inspect" && /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-4", children: inspectLoading ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-8 text-center", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(RefreshCw, { className: "h-6 w-6 animate-spin mx-auto mb-2 text-gray-400" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500", children: "Loading intercepted traffic..." })
    ] }) : inspectError ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-8 text-center text-red-600", children: [
      "Failed to load intercepted traffic. ",
      canRetry && /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { onClick: retry, children: "Retry" })
    ] }) : (inspectData == null ? void 0 : inspectData.message) ? /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-8 text-center", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsx(Code, { className: "h-12 w-12 mx-auto mb-4 text-gray-400" }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-gray-600 dark:text-gray-400", children: inspectData.message }),
      /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500 mt-2", children: "Request/response inspection will be available in a future version." })
    ] }) : /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold mb-2", children: "Intercepted Requests" }),
        (inspectData == null ? void 0 : inspectData.requests.length) === 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500", children: "No requests intercepted yet." }) : /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2", children: inspectData == null ? void 0 : inspectData.requests.map((req) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "div",
          {
            className: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 mb-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(Badge$1, { variant: "info", children: req.method }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("code", { className: "text-sm", children: req.url }),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-xs text-gray-500", children: req.timestamp })
              ] }),
              req.body && /* @__PURE__ */ jsxRuntimeExports.jsx("pre", { className: "text-xs bg-gray-50 dark:bg-gray-900 p-2 rounded mt-2 overflow-x-auto", children: req.body })
            ]
          },
          req.id
        )) })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-lg font-semibold mb-2", children: "Intercepted Responses" }),
        (inspectData == null ? void 0 : inspectData.responses.length) === 0 ? /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-sm text-gray-500", children: "No responses intercepted yet." }) : /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-2", children: inspectData == null ? void 0 : inspectData.responses.map((res) => /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "div",
          {
            className: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2 mb-2", children: [
                /* @__PURE__ */ jsxRuntimeExports.jsx(
                  Badge$1,
                  {
                    variant: res.status_code >= 200 && res.status_code < 300 ? "success" : res.status_code >= 400 && res.status_code < 500 ? "warning" : "danger",
                    children: res.status_code
                  }
                ),
                /* @__PURE__ */ jsxRuntimeExports.jsx("span", { className: "text-xs text-gray-500", children: res.timestamp })
              ] }),
              res.body && /* @__PURE__ */ jsxRuntimeExports.jsx("pre", { className: "text-xs bg-gray-50 dark:bg-gray-900 p-2 rounded mt-2 overflow-x-auto", children: res.body })
            ]
          },
          res.id
        )) })
      ] })
    ] }) }) }) })
  ] });
}
function ProxyRuleForm({ rule, onSave, onCancel }) {
  const [formData, setFormData] = reactExports.useState({
    pattern: (rule == null ? void 0 : rule.pattern) || "",
    type: (rule == null ? void 0 : rule.type) || "request",
    status_codes: (rule == null ? void 0 : rule.status_codes) || [],
    body_transforms: (rule == null ? void 0 : rule.body_transforms) || [{ path: "", replace: "", operation: "replace" }],
    enabled: (rule == null ? void 0 : rule.enabled) ?? true
  });
  const handleSubmit = (e) => {
    e.preventDefault();
    onSave(formData);
  };
  const addTransform = () => {
    setFormData({
      ...formData,
      body_transforms: [
        ...formData.body_transforms,
        { path: "", replace: "", operation: "replace" }
      ]
    });
  };
  const removeTransform = (index) => {
    setFormData({
      ...formData,
      body_transforms: formData.body_transforms.filter((_, i) => i !== index)
    });
  };
  const updateTransform = (index, field, value) => {
    const updated = [...formData.body_transforms];
    updated[index] = { ...updated[index], [field]: value };
    setFormData({ ...formData, body_transforms: updated });
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "fixed inset-0 bg-black/50 flex items-center justify-center z-50", children: /* @__PURE__ */ jsxRuntimeExports.jsx(Card, { className: "w-full max-w-2xl max-h-[90vh] overflow-y-auto m-4", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-6", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx("h3", { className: "text-xl font-semibold mb-4", children: rule ? "Edit Proxy Rule" : "Create Proxy Rule" }),
    /* @__PURE__ */ jsxRuntimeExports.jsxs("form", { onSubmit: handleSubmit, className: "space-y-4", children: [
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium mb-1", children: "URL Pattern" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Input$1,
          {
            value: formData.pattern,
            onChange: (e) => setFormData({ ...formData, pattern: e.target.value }),
            placeholder: "/api/users/*",
            required: true
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-xs text-gray-500 mt-1", children: "Supports wildcards (e.g., /api/users/*)" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium mb-1", children: "Rule Type" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(
          "select",
          {
            value: formData.type,
            onChange: (e) => setFormData({ ...formData, type: e.target.value }),
            className: "w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800",
            children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "request", children: "Request" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "response", children: "Response" })
            ]
          }
        )
      ] }),
      formData.type === "response" && /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium mb-1", children: "Status Codes (comma-separated)" }),
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          Input$1,
          {
            value: formData.status_codes.join(", "),
            onChange: (e) => {
              const codes = e.target.value.split(",").map((s) => parseInt(s.trim())).filter((n) => !isNaN(n));
              setFormData({ ...formData, status_codes: codes });
            },
            placeholder: "200, 201, 404"
          }
        )
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-sm font-medium mb-2", children: "Body Transformations" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "space-y-3", children: [
          formData.body_transforms.map((transform, index) => /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "p-3 border border-gray-200 dark:border-gray-700 rounded-lg space-y-2", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center justify-between mb-2", children: [
              /* @__PURE__ */ jsxRuntimeExports.jsxs("span", { className: "text-sm font-medium", children: [
                "Transform ",
                index + 1
              ] }),
              formData.body_transforms.length > 1 && /* @__PURE__ */ jsxRuntimeExports.jsx(
                Button$1,
                {
                  type: "button",
                  variant: "ghost",
                  size: "sm",
                  onClick: () => removeTransform(index),
                  children: /* @__PURE__ */ jsxRuntimeExports.jsx(Trash2, { className: "h-4 w-4" })
                }
              )
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs font-medium mb-1", children: "JSONPath" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  value: transform.path,
                  onChange: (e) => updateTransform(index, "path", e.target.value),
                  placeholder: "$.userId",
                  required: true
                }
              )
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs font-medium mb-1", children: "Replacement Value" }),
              /* @__PURE__ */ jsxRuntimeExports.jsx(
                Input$1,
                {
                  value: transform.replace,
                  onChange: (e) => updateTransform(index, "replace", e.target.value),
                  placeholder: "{{uuid}}",
                  required: true
                }
              ),
              /* @__PURE__ */ jsxRuntimeExports.jsxs("p", { className: "text-xs text-gray-500 mt-1", children: [
                "Supports templates: ",
                "{{",
                "uuid",
                "}}",
                ", ",
                "{{",
                "faker.email",
                "}}",
                ", etc."
              ] })
            ] }),
            /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { children: [
              /* @__PURE__ */ jsxRuntimeExports.jsx("label", { className: "block text-xs font-medium mb-1", children: "Operation" }),
              /* @__PURE__ */ jsxRuntimeExports.jsxs(
                "select",
                {
                  value: transform.operation,
                  onChange: (e) => updateTransform(index, "operation", e.target.value),
                  className: "w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-sm",
                  children: [
                    /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "replace", children: "Replace" }),
                    /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "add", children: "Add" }),
                    /* @__PURE__ */ jsxRuntimeExports.jsx("option", { value: "remove", children: "Remove" })
                  ]
                }
              )
            ] })
          ] }, index)),
          /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { type: "button", variant: "outline", onClick: addTransform, className: "w-full", children: [
            /* @__PURE__ */ jsxRuntimeExports.jsx(Plus, { className: "h-4 w-4 mr-2" }),
            "Add Transformation"
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(
          "input",
          {
            type: "checkbox",
            id: "enabled",
            checked: formData.enabled,
            onChange: (e) => setFormData({ ...formData, enabled: e.target.checked }),
            className: "h-4 w-4"
          }
        ),
        /* @__PURE__ */ jsxRuntimeExports.jsx("label", { htmlFor: "enabled", className: "text-sm font-medium", children: "Enabled" })
      ] }),
      /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "flex justify-end gap-2 pt-4 border-t", children: [
        /* @__PURE__ */ jsxRuntimeExports.jsx(Button$1, { type: "button", variant: "outline", onClick: onCancel, children: "Cancel" }),
        /* @__PURE__ */ jsxRuntimeExports.jsxs(Button$1, { type: "submit", children: [
          rule ? "Update" : "Create",
          " Rule"
        ] })
      ] })
    ] })
  ] }) }) });
}
function ProxyInspectorPage() {
  return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "container mx-auto px-4 py-6", children: /* @__PURE__ */ jsxRuntimeExports.jsx(ProxyInspector, {}) });
}
const DashboardPage = reactExports.lazy(() => __vitePreload(() => import("./DashboardPage.CNMptHoX.js"), true ? __vite__mapDeps([0,1,2,3,4]) : void 0).then((m) => ({ default: m.DashboardPage })));
const ServicesPage = reactExports.lazy(() => __vitePreload(() => import("./ServicesPage.CmiICeQ4.js"), true ? __vite__mapDeps([5,1,4]) : void 0).then((m) => ({ default: m.ServicesPage })));
const LogsPage = reactExports.lazy(() => __vitePreload(() => import("./LogsPage.CJxt3GcC.js"), true ? __vite__mapDeps([6,1,7,8,9,4]) : void 0).then((m) => ({ default: m.LogsPage })));
const MetricsPage = reactExports.lazy(() => __vitePreload(() => import("./MetricsPage.Cw2KF7jM.js"), true ? __vite__mapDeps([10,1,4]) : void 0).then((m) => ({ default: m.MetricsPage })));
const VerificationPage = reactExports.lazy(() => __vitePreload(() => import("./VerificationPage.C-JPS3kd.js"), true ? __vite__mapDeps([11,1,4]) : void 0).then((m) => ({ default: m.VerificationPage })));
const ContractDiffPage = reactExports.lazy(() => __vitePreload(() => import("./ContractDiffPage.Bl79VmwU.js"), true ? __vite__mapDeps([12,1,13,9,4]) : void 0).then((m) => ({ default: m.ContractDiffPage })));
const IncidentDashboardPage = reactExports.lazy(() => __vitePreload(() => import("./IncidentDashboardPage.BTB6c-IS.js"), true ? __vite__mapDeps([14,1,4]) : void 0).then((m) => ({ default: m.IncidentDashboardPage })));
const FitnessFunctionsPage = reactExports.lazy(() => __vitePreload(() => import("./FitnessFunctionsPage.DI4hfpBK.js"), true ? __vite__mapDeps([15,1,16,17,4]) : void 0).then((m) => ({ default: m.FitnessFunctionsPage })));
const FixturesPage = reactExports.lazy(() => __vitePreload(() => import("./FixturesPage.B-8YdD7N.js"), true ? __vite__mapDeps([18,1,4]) : void 0).then((m) => ({ default: m.FixturesPage })));
const TestingPage = reactExports.lazy(() => __vitePreload(() => import("./TestingPage.De-Fcur8.js"), true ? __vite__mapDeps([19,1,4]) : void 0).then((m) => ({ default: m.TestingPage })));
const ImportPage = reactExports.lazy(() => __vitePreload(() => import("./ImportPage.DpPkHDuE.js"), true ? __vite__mapDeps([20,1,4]) : void 0).then((m) => ({ default: m.ImportPage })));
const WorkspacesPage = reactExports.lazy(() => __vitePreload(() => import("./WorkspacesPage.CA4oueEK.js"), true ? __vite__mapDeps([21,1,17,4]) : void 0));
const PlaygroundPage = reactExports.lazy(() => __vitePreload(() => import("./PlaygroundPage.GjwNBDPC.js"), true ? __vite__mapDeps([22,1,7,8,9,4]) : void 0).then((m) => ({ default: m.PlaygroundPage })));
const PluginsPage = reactExports.lazy(() => __vitePreload(() => import("./PluginsPage.pREEuALG.js"), true ? __vite__mapDeps([23,1,4]) : void 0).then((m) => ({ default: m.PluginsPage })));
const ChainsPage = reactExports.lazy(() => __vitePreload(() => import("./ChainsPage.Cr8o0PYP.js"), true ? __vite__mapDeps([24,1,4]) : void 0).then((m) => ({ default: m.ChainsPage })));
const GraphPage = reactExports.lazy(() => __vitePreload(() => import("./GraphPage.XVoJOfyW.js"), true ? __vite__mapDeps([25,1,26,27,28,29,4]) : void 0).then((m) => ({ default: m.GraphPage })));
const WorldStatePage = reactExports.lazy(() => __vitePreload(() => import("./WorldStatePage.B4Uz0fNH.js"), true ? __vite__mapDeps([30,1,26,27,29,31,3,4]) : void 0).then((m) => ({ default: m.WorldStatePage })));
const PerformancePage = reactExports.lazy(() => __vitePreload(() => import("./PerformancePage.QCDrGEz6.js"), true ? __vite__mapDeps([32,1,33,4]) : void 0).then((m) => ({ default: m.default })));
const ScenarioStateMachineEditor = reactExports.lazy(() => __vitePreload(() => import("./ScenarioStateMachineEditor.c_q_v59Z.js"), true ? __vite__mapDeps([34,1,26,27,31,35,33,4]) : void 0).then((m) => ({ default: m.ScenarioStateMachineEditor })));
const ScenarioStudioPage = reactExports.lazy(() => __vitePreload(() => import("./ScenarioStudioPage.5sALfbGz.js"), true ? __vite__mapDeps([36,1,26,27,33,35,31,4]) : void 0).then((m) => ({ default: m.ScenarioStudioPage })));
const AnalyticsPage = reactExports.lazy(() => __vitePreload(() => import("./AnalyticsPage.B5ytAR1q.js"), true ? __vite__mapDeps([37,1,2,4]) : void 0));
const PillarAnalyticsPage = reactExports.lazy(() => __vitePreload(() => import("./PillarAnalyticsPage.2cpxnFW_.js"), true ? __vite__mapDeps([38,1,8,2,4]) : void 0).then((m) => ({ default: m.PillarAnalyticsPage })));
const HostedMocksPage = reactExports.lazy(() => __vitePreload(() => import("./HostedMocksPage.CGb2jbHx.js"), true ? __vite__mapDeps([39,1]) : void 0).then((m) => ({ default: m.HostedMocksPage })));
const ObservabilityPage = reactExports.lazy(() => __vitePreload(() => import("./ObservabilityPage.Cothqdf2.js"), true ? __vite__mapDeps([40,1,31,4]) : void 0).then((m) => ({ default: m.ObservabilityPage })));
const TracesPage = reactExports.lazy(() => __vitePreload(() => import("./TracesPage.BFT8tZRL.js"), true ? __vite__mapDeps([41,1,4]) : void 0).then((m) => ({ default: m.TracesPage })));
const TestGeneratorPage = reactExports.lazy(() => __vitePreload(() => import("./TestGeneratorPage.DqTtvLl4.js"), true ? __vite__mapDeps([42,1]) : void 0));
const TestExecutionDashboard = reactExports.lazy(() => __vitePreload(() => import("./TestExecutionDashboard.B6F-0BL5.js"), true ? __vite__mapDeps([43,1,27]) : void 0));
const IntegrationTestBuilder = reactExports.lazy(() => __vitePreload(() => import("./IntegrationTestBuilder.D6vXxWj-.js"), true ? __vite__mapDeps([44,1]) : void 0));
const ChaosPage = reactExports.lazy(() => __vitePreload(() => import("./ChaosPage.TYn4CA_q.js"), true ? __vite__mapDeps([45,1,3,2,33,4]) : void 0).then((m) => ({ default: m.ChaosPage })));
const ResiliencePage = reactExports.lazy(() => __vitePreload(() => import("./ResiliencePage.BbdnN0sI.js"), true ? __vite__mapDeps([46,1]) : void 0).then((m) => ({ default: m.ResiliencePage })));
const RecorderPage = reactExports.lazy(() => __vitePreload(() => import("./RecorderPage.3kwHtLUF.js"), true ? __vite__mapDeps([47,1,4]) : void 0).then((m) => ({ default: m.RecorderPage })));
const BehavioralCloningPage = reactExports.lazy(() => __vitePreload(() => import("./BehavioralCloningPage.CGZR3RJK.js"), true ? __vite__mapDeps([48,1,16,9,8,49,4]) : void 0).then((m) => ({ default: m.BehavioralCloningPage })));
const OrchestrationBuilder = reactExports.lazy(() => __vitePreload(() => import("./OrchestrationBuilder.D2VUNLtZ.js"), true ? __vite__mapDeps([50,1]) : void 0));
const OrchestrationExecutionView = reactExports.lazy(() => __vitePreload(() => import("./OrchestrationExecutionView.BMkFSF7c.js"), true ? __vite__mapDeps([51,1,31,4]) : void 0));
const PluginRegistryPage = reactExports.lazy(() => __vitePreload(() => import("./PluginRegistryPage.BChYShpO.js"), true ? __vite__mapDeps([52,1,4]) : void 0));
const TemplateMarketplacePage = reactExports.lazy(() => __vitePreload(() => import("./TemplateMarketplacePage.0JBAO_OF.js"), true ? __vite__mapDeps([53,1]) : void 0));
const ShowcasePage = reactExports.lazy(() => __vitePreload(() => import("./ShowcasePage.BNPMnQ99.js"), true ? __vite__mapDeps([54,1,55,4]) : void 0).then((m) => ({ default: m.ShowcasePage })));
const LearningHubPage = reactExports.lazy(() => __vitePreload(() => import("./LearningHubPage.DH98z8lH.js"), true ? __vite__mapDeps([56,1,55,4]) : void 0).then((m) => ({ default: m.LearningHubPage })));
const UserManagementPage = reactExports.lazy(() => __vitePreload(() => import("./UserManagementPage.VW349Hw_.js"), true ? __vite__mapDeps([57,1,58,4]) : void 0).then((m) => ({ default: m.UserManagementPage })));
const MockAIPage = reactExports.lazy(() => __vitePreload(() => import("./MockAIPage.5hxwO1jM.js"), true ? __vite__mapDeps([59,1,13,8,4]) : void 0).then((m) => ({ default: m.MockAIPage })));
const MockAIOpenApiGeneratorPage = reactExports.lazy(() => __vitePreload(() => import("./MockAIOpenApiGeneratorPage.Ch-B5TAz.js"), true ? __vite__mapDeps([60,1,4]) : void 0).then((m) => ({ default: m.MockAIOpenApiGeneratorPage })));
const MockAIRulesPage = reactExports.lazy(() => __vitePreload(() => import("./MockAIRulesPage.FJlsCT58.js"), true ? __vite__mapDeps([61,1,8,4]) : void 0).then((m) => ({ default: m.MockAIRulesPage })));
const VoicePage = reactExports.lazy(() => __vitePreload(() => import("./VoicePage.BtpGkUJl.js"), true ? __vite__mapDeps([62,1,13,9,8,4]) : void 0).then((m) => ({ default: m.VoicePage })));
const AIStudioPage = reactExports.lazy(() => __vitePreload(() => import("./AIStudioPage.BPIQUcfV.js"), true ? __vite__mapDeps([63,1,13,28,8,49,58,4]) : void 0).then((m) => ({ default: m.AIStudioPage })));
function App() {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = reactExports.useState("dashboard");
  const loadWorkspaces = useWorkspaceStore((state) => state.loadWorkspaces);
  useStartupPrefetch();
  reactExports.useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);
  reactExports.useEffect(() => {
    const handleNavigate = (event) => {
      const { target, id } = event.detail;
      if (target === "chaos") {
        setActiveTab("chaos");
      } else if (target === "scenario") {
        setActiveTab("scenario-studio");
      } else if (target === "persona") {
        setActiveTab("ai-studio");
      }
    };
    const handleNavigateTab = (event) => {
      const { tab } = event.detail;
      if (tab) {
        setActiveTab(tab);
      }
    };
    window.addEventListener("navigate", handleNavigate);
    window.addEventListener("navigate-tab", handleNavigateTab);
    return () => {
      window.removeEventListener("navigate", handleNavigate);
      window.removeEventListener("navigate-tab", handleNavigateTab);
    };
  }, []);
  reactExports.useEffect(() => {
    __vitePreload(async () => {
      const { isTauri, listenToTauriEvent } = await import("./tauri.UVgQf7G2.js");
      return { isTauri, listenToTauriEvent };
    }, true ? __vite__mapDeps([64,1,4]) : void 0).then(({ isTauri, listenToTauriEvent }) => {
      if (isTauri) {
        const cleanup1 = listenToTauriEvent("file-opened", (filePath) => {
          __vitePreload(async () => {
            const { handleFileOpen } = await import("./tauri.UVgQf7G2.js");
            return { handleFileOpen };
          }, true ? __vite__mapDeps([64,1,4]) : void 0).then(({ handleFileOpen }) => {
            handleFileOpen(filePath).catch((err) => {
              console.error("Failed to handle file open:", err);
            });
          });
        });
        const cleanup2 = listenToTauriEvent("file-dropped", (filePath) => {
          __vitePreload(async () => {
            const { handleFileOpen } = await import("./tauri.UVgQf7G2.js");
            return { handleFileOpen };
          }, true ? __vite__mapDeps([64,1,4]) : void 0).then(({ handleFileOpen }) => {
            handleFileOpen(filePath).catch((err) => {
              console.error("Failed to handle file drop:", err);
            });
          });
        });
        const cleanup3 = listenToTauriEvent("config-file-opened", (configContent) => {
        });
        return () => {
          cleanup1();
          cleanup2();
          cleanup3();
        };
      }
    });
  }, []);
  const handleRefresh = () => {
  };
  const renderPage = () => {
    switch (activeTab) {
      // Core
      case "dashboard":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(DashboardPage, {});
      case "workspaces":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(WorkspacesPage, {});
      case "playground":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(PlaygroundPage, {});
      case "federation":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(FederationPage, {});
      // Services & Data
      case "services":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ServicesPage, {});
      case "virtual-backends":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(VirtualBackendsPage, {});
      case "fixtures":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(FixturesPage, {});
      case "hosted-mocks":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(HostedMocksPage, {});
      case "tunnels":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TunnelsPage, {});
      // Orchestration
      case "chains":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ChainsPage, {});
      case "graph":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(GraphPage, {});
      case "world-state":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(WorldStatePage, {});
      case "performance":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(PerformancePage, {});
      case "state-machine-editor":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ScenarioStateMachineEditor, {});
      case "scenario-studio":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ScenarioStudioPage, {});
      case "orchestration-builder":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(OrchestrationBuilder, {});
      case "orchestration-execution":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(OrchestrationExecutionView, { orchestrationId: "default" });
      // Observability & Monitoring
      case "observability":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ObservabilityPage, {});
      case "status":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(StatusPage, {});
      case "logs":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(LogsPage, {});
      case "traces":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TracesPage, {});
      case "metrics":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(MetricsPage, {});
      case "analytics":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(AnalyticsPage, {});
      case "pillar-analytics":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(PillarAnalyticsPage, {});
      case "verification":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(VerificationPage, {});
      case "contract-diff":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ContractDiffPage, {});
      case "incidents":
      case "incident-dashboard":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(IncidentDashboardPage, {});
      case "fitness-functions":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(FitnessFunctionsPage, {});
      // Testing
      case "testing":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TestingPage, {});
      case "test-generator":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TestGeneratorPage, {});
      case "test-execution":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TestExecutionDashboard, {});
      case "integration-test-builder":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(IntegrationTestBuilder, {});
      // Chaos & Resilience
      case "chaos":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ChaosPage, {});
      case "resilience":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ResiliencePage, {});
      case "recorder":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(RecorderPage, {});
      case "behavioral-cloning":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(BehavioralCloningPage, {});
      // Import & Templates
      case "import":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ImportPage, {});
      case "template-marketplace":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TemplateMarketplacePage, {});
      // Community Portal
      case "showcase":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ShowcasePage, {});
      case "learning-hub":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(LearningHubPage, {});
      // Plugins
      case "plugins":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(PluginsPage, {});
      case "plugin-registry":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(PluginRegistryPage, {});
      // User Management
      case "user-management":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(UserManagementPage, {});
      // MockAI
      case "mockai":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(MockAIPage, {});
      case "mockai-openapi-generator":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(MockAIOpenApiGeneratorPage, {});
      case "mockai-rules":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(MockAIRulesPage, {});
      // Voice + LLM Interface
      case "voice":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(VoicePage, {});
      // AI Studio - Unified AI Copilot
      case "ai-studio":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(AIStudioPage, {});
      // Configuration
      case "config":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ConfigPage, {});
      case "organization":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(OrganizationPage, {});
      case "billing":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(BillingPage, {});
      case "api-tokens":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ApiTokensPage, {});
      case "byok":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(BYOKConfigPage, {});
      case "usage":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(UsageDashboardPage, {});
      // Time Travel
      case "time-travel":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(TimeTravelPage, {});
      // Proxy Inspector
      case "proxy-inspector":
        return /* @__PURE__ */ jsxRuntimeExports.jsx(ProxyInspectorPage, {});
      default:
        return /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "space-y-8", children: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-center py-12", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center", children: [
          /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "text-6xl mb-4", children: "🚧" }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("h2", { className: "text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2", children: t("app.pageNotFoundTitle") }),
          /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "text-gray-600 dark:text-gray-400 mb-6", children: t("app.pageNotFoundBody") }),
          /* @__PURE__ */ jsxRuntimeExports.jsx(
            "button",
            {
              onClick: () => setActiveTab("dashboard"),
              className: "px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors",
              children: t("app.goToDashboard")
            }
          )
        ] }) }) });
    }
  };
  return /* @__PURE__ */ jsxRuntimeExports.jsx(ErrorBoundary, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(ToastProvider, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(AuthGuard, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(AppShell, { activeTab, onTabChange: setActiveTab, onRefresh: handleRefresh, children: /* @__PURE__ */ jsxRuntimeExports.jsx(ErrorBoundary, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(reactExports.Suspense, { fallback: /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "flex items-center justify-center h-64", children: /* @__PURE__ */ jsxRuntimeExports.jsxs("div", { className: "text-center", children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx("div", { className: "inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" }),
    /* @__PURE__ */ jsxRuntimeExports.jsx("p", { className: "mt-4 text-gray-600 dark:text-gray-400", children: t("app.loading") })
  ] }) }), children: renderPage() }) }) }) }) }) });
}
const isLocalhost = Boolean(
  window.location.hostname === "localhost" || window.location.hostname === "[::1]" || window.location.hostname.match(/^127(?:\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)){3}$/)
);
const SW_VERSION = "v3";
function registerServiceWorker(config) {
  if (!("serviceWorker" in navigator)) return;
  const publicUrl = new URL("/", window.location.href);
  if (publicUrl.origin !== window.location.origin) {
    return;
  }
  const swUrl = `${"/"}sw.js?version=${SW_VERSION}`;
  const clearStaleServiceWorkers = async () => {
    const registrations = await navigator.serviceWorker.getRegistrations();
    await Promise.all(
      registrations.map(async (registration) => {
        var _a, _b, _c;
        const url = ((_a = registration.active) == null ? void 0 : _a.scriptURL) || ((_b = registration.installing) == null ? void 0 : _b.scriptURL) || ((_c = registration.waiting) == null ? void 0 : _c.scriptURL);
        if (url && !url.includes(`version=${SW_VERSION}`)) {
          try {
            await registration.unregister();
          } catch (err) {
            console.warn("[Service Worker] Failed to unregister stale registration", err);
          }
        }
      })
    );
    const cacheNames = await caches.keys();
    await Promise.all(
      cacheNames.map((name) => {
        if (!name.includes(SW_VERSION)) {
          return caches.delete(name);
        }
        return Promise.resolve(false);
      })
    );
  };
  window.addEventListener("load", () => {
    clearStaleServiceWorkers().catch((err) => {
      console.warn("[Service Worker] Failed to clear stale registrations", err);
    });
    if (isLocalhost) {
      checkValidServiceWorker(swUrl, config);
      navigator.serviceWorker.ready.then(() => {
        console.log("[Service Worker] Ready on localhost");
      });
    } else {
      registerValidSW(swUrl, config);
    }
  });
}
function registerValidSW(swUrl, config) {
  navigator.serviceWorker.register(swUrl).then((registration) => {
    registration.onupdatefound = () => {
      const installingWorker = registration.installing;
      if (installingWorker == null) {
        return;
      }
      installingWorker.onstatechange = () => {
        var _a;
        if (installingWorker.state === "installed") {
          if (navigator.serviceWorker.controller) {
            console.log("[Service Worker] New content available; please refresh.");
            if (config && config.onUpdate) {
              config.onUpdate(registration);
            } else {
              (_a = registration.waiting) == null ? void 0 : _a.postMessage({ type: "SKIP_WAITING" });
              window.location.reload();
            }
          } else {
            console.log("[Service Worker] Content cached for offline use.");
            if (config && config.onSuccess) {
              config.onSuccess(registration);
            }
          }
        }
      };
    };
  }).catch((error) => {
    console.error("[Service Worker] Registration failed:", error);
  });
}
function checkValidServiceWorker(swUrl, config) {
  fetch(swUrl, {
    headers: { "Service-Worker": "script" }
  }).then((response) => {
    const contentType = response.headers.get("content-type");
    if (response.status === 404 || contentType != null && contentType.indexOf("javascript") === -1) {
      navigator.serviceWorker.ready.then((registration) => {
        registration.unregister();
      });
    } else {
      registerValidSW(swUrl, config);
    }
  }).catch(() => {
    console.log("[Service Worker] No internet connection found. App is running in offline mode.");
  });
}
const ReactQueryDevtools = null;
useThemePaletteStore.getState().init();
void initErrorReporting();
{
  registerServiceWorker({
    onSuccess: (registration) => {
      console.log("[PWA] Service worker registered successfully");
      logger.info("PWA: Service worker registered", { registration });
    },
    onUpdate: (registration) => {
      var _a;
      console.log("[PWA] New service worker available");
      logger.info("PWA: New service worker available", { registration });
      if (window.confirm("New version available! Reload to update?")) {
        (_a = registration.waiting) == null ? void 0 : _a.postMessage({ type: "SKIP_WAITING" });
        window.location.reload();
      }
    }
  });
}
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error) => {
        if ((error == null ? void 0 : error.status) && error.status >= 400 && error.status < 500) {
          return false;
        }
        return failureCount < 3;
      },
      retryDelay: (attemptIndex) => Math.min(1e3 * 2 ** attemptIndex, 3e4),
      staleTime: 3e4,
      // 30 seconds - data considered fresh
      gcTime: 10 * 60 * 1e3,
      // 10 minutes - keep in cache longer
      refetchOnWindowFocus: false,
      // Disable to reduce network requests
      refetchOnReconnect: true,
      // Refetch when connection restored
      refetchOnMount: true,
      // Always refetch on mount for fresh data
      networkMode: "online"
      // Only run queries when online
    },
    mutations: {
      retry: (failureCount, error) => {
        if ((error == null ? void 0 : error.status) && error.status >= 400 && error.status < 500) {
          return false;
        }
        return failureCount < 2;
      },
      retryDelay: 1e3,
      networkMode: "online"
    }
  }
});
clientExports.createRoot(document.getElementById("root")).render(
  /* @__PURE__ */ jsxRuntimeExports.jsx(reactExports.StrictMode, { children: /* @__PURE__ */ jsxRuntimeExports.jsx(QueryClientProvider, { client: queryClient, children: /* @__PURE__ */ jsxRuntimeExports.jsxs(I18nProvider, { children: [
    /* @__PURE__ */ jsxRuntimeExports.jsx(App, {}),
    ReactQueryDevtools
  ] }) }) })
);
export {
  StatusIcon$1 as $,
  Alert as A,
  Button$1 as B,
  Card as C,
  Database as D,
  EmptyState as E,
  Funnel as F,
  Globe as G,
  RotateCcw as H,
  Input$1 as I,
  FastForward as J,
  Settings as K,
  Slider as L,
  ModernCard as M,
  Calendar as N,
  useRealityShortcuts as O,
  Play as P,
  PageHeader as Q,
  RefreshCw as R,
  SkeletonCard as S,
  TriangleAlert as T,
  User as U,
  RealityIndicator as V,
  Section as W,
  RealitySlider as X,
  MetricCard as Y,
  Zap as Z,
  MetricIcon as _,
  useErrorToast as a,
  TabsContent as a$,
  logger as a0,
  Switch as a1,
  ContextMenuWithItems as a2,
  useServiceStore as a3,
  CircleAlert as a4,
  Search as a5,
  Download as a6,
  Eye as a7,
  ChevronDown as a8,
  useMetrics as a9,
  Code as aA,
  useDriftIncidents as aB,
  useDriftIncidentStatistics as aC,
  useUpdateDriftIncident as aD,
  useResolveDriftIncident as aE,
  TestTube as aF,
  Dialog as aG,
  DialogContent as aH,
  DialogHeader as aI,
  DialogTitle as aJ,
  DialogDescription as aK,
  DialogFooter as aL,
  SquarePen as aM,
  useFixtures as aN,
  DialogClose as aO,
  ue as aP,
  CircleCheckBig as aQ,
  dashboardApi as aR,
  smokeTestsApi as aS,
  usePreviewImport as aT,
  useImportPostman as aU,
  useImportInsomnia as aV,
  useImportCurl as aW,
  Tabs as aX,
  TabsList as aY,
  TabsTrigger as aZ,
  History as a_,
  Activity as aa,
  TrendingUp as ab,
  ChartColumn as ac,
  Label as ad,
  Select as ae,
  SelectTrigger as af,
  SelectValue as ag,
  SelectContent as ah,
  SelectItem as ai,
  Textarea as aj,
  CircleCheck as ak,
  CircleX as al,
  verificationApi as am,
  authenticatedFetch as an,
  X as ao,
  Trash2 as ap,
  Plus as aq,
  Save as ar,
  Network as as,
  contractDiffApi as at,
  createLucideIcon as au,
  Users as av,
  driftApi as aw,
  ExternalLink as ax,
  ChevronRight as ay,
  Package as az,
  useDashboard as b,
  useUpdateChaosLatency as b$,
  Button as b0,
  Card$1 as b1,
  useImportHistory as b2,
  useClearImportHistory as b3,
  Upload as b4,
  CardHeader as b5,
  CardTitle as b6,
  CardDescription as b7,
  CardContent as b8,
  apiService as b9,
  Link as bA,
  Label$1 as bB,
  Input as bC,
  useI18n as bD,
  pluginsApi as bE,
  Puzzle as bF,
  Mail as bG,
  Radio as bH,
  PanelsTopLeft as bI,
  __vitePreload as bJ,
  ArrowRight as bK,
  Layers as bL,
  useConnectionStore as bM,
  Cloud as bN,
  Brain as bO,
  useChaosLatencyMetrics as bP,
  useChaosLatencyStats as bQ,
  useUpdateErrorPattern as bR,
  useNetworkProfiles as bS,
  useApplyNetworkProfile as bT,
  useCreateNetworkProfile as bU,
  useDeleteNetworkProfile as bV,
  Wifi as bW,
  useChaosConfig as bX,
  useExportNetworkProfile as bY,
  useImportNetworkProfile as bZ,
  useChaosStatus as b_,
  Shield as ba,
  Lock as bb,
  Key as bc,
  EyeOff as bd,
  Copy as be,
  DialogTrigger as bf,
  useWorkspaceStore as bg,
  useUpdateWorkspacesOrder as bh,
  importApi as bi,
  Checkbox as bj,
  FolderOpen as bk,
  GripVertical as bl,
  LoaderCircle as bm,
  Info as bn,
  FileJson as bo,
  Check as bp,
  CircleQuestionMark as bq,
  CodeXml as br,
  GitBranch as bs,
  DropdownMenu as bt,
  DropdownMenuTrigger as bu,
  DropdownMenuContent as bv,
  DropdownMenuItem as bw,
  Modal as bx,
  HardDrive as by,
  Progress as bz,
  Server as c,
  useUpdateChaosFaults as c0,
  useUpdateChaosTraffic as c1,
  useResetChaos as c2,
  SkeletonMetricCard as c3,
  SkeletonChart as c4,
  House as c5,
  Mic as c6,
  GitCompare as c7,
  useToast as c8,
  BookOpen as c9,
  Building2 as ca,
  cn as d,
  ModernBadge as e,
  useApiErrorHandling as f,
  useClearLogs as g,
  useLogs as h,
  SkeletonTable as i,
  DataErrorFallback as j,
  ResponsiveTable as k,
  FileText as l,
  Badge$1 as m,
  Clock as n,
  useTimeTravelStatus as o,
  useEnableTimeTravel as p,
  useDisableTimeTravel as q,
  useAdvanceTime as r,
  useSetTime as s,
  useSetTimeScale as t,
  usePreferencesStore as u,
  useResetTimeTravel as v,
  useLivePreviewLifecycleUpdates as w,
  Badge as x,
  Pause as y,
  Tooltip as z
};
//# sourceMappingURL=index.js.map
