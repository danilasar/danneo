<?php
/**
 * Project:     CMS Danneo : Content management system
 * File:        apanel.template.php
 *
 * Класс шаблона
 *
 * @version	 $Id: Danneo CMS v.0.5.5 Release $
 * @package      CMS Danneo basis kernel
 * @copyright    Copyright (C) 2004 - 2013 Danneo Team. All rights reserved.
 * @link         http://danneo.com, http://danneo.ru
 * @license      http://www.gnu.org/licenses/gpl-2.0.html   GNU General Public License, version 2
 */
if (!defined("ADMREAD")) exit(); 

class tm
{
    //var $content = '';
    //var $explain = '';   
       
   /**
    * Шапка шаблона оформления 
    */
    function globalstart() {
        global $a,$conf,$mods,$sess,$wysiwyg,$lang,$_COOKIE,$PLATFORM,$ADMIN_ID,$CHECK_ADMIN,$ADMIN_PERM_ARRAY,$_REQUEST,$panelimg,$style; 
        $sess['skin'] = (is_dir(ADMINDIR.'/template/'.$sess['skin'])) ? $sess['skin'] : SKIN_DEF;

        if ( stristr(USER_AGENT, 'MSIE 6.0') OR stristr(USER_AGENT, 'MSIE 7.0') != false) { 
            //if (!isset($_COOKIE['ie'])) { 
            //    setcookie('ie', 'yes', time() + 606024); 
                redirect("login.php?opsss=4");
            //} 
        }         
        if (isset($_COOKIE['menup']) && $_COOKIE['menup']=='closed') {
            $panelimg = "<img id=\"menup\" src=\"template/".$sess['skin']."/images/open.gif\" alt=\"".$lang['openclose']."\" />";
            $style = 'display: none;';
        } else {
            $panelimg = "<img id=\"menup\" src=\"template/".$sess['skin']."/images/closed.gif\" alt=\"".$lang['openclose']."\" />";
            $style = '';
        }   
        $a = (defined('ENABLE_AJAX') && ENABLE_AJAX == 'yes') ? 1 : 0; 
        echo '<!DOCTYPE html>
              <html lang='.$conf['langcode'].'>
              <head>
              <meta charset="'.$conf['langcharset'].'">
              <title>CMS Danneo '.$conf['version'].'</title>       
              <link rel="stylesheet" href="template/'.$sess['skin'].'/css/template.css?v.'.VERSION.'">
              <link rel="stylesheet" href="template/'.$sess['skin'].'/css/filebrowser.css">
              <link rel="stylesheet" href="template/'.$sess['skin'].'/css/calendar/calendar.css" />
              <link rel="stylesheet" href="template/'.$sess['skin'].'/css/colorbox/colorbox.css" />
              <link rel="stylesheet" href="template/'.$sess['skin'].'/css/colorpicker.css" />
              <script src="javascript/jquery.js"></script>    
              <script src="javascript/script.js"></script>
              <script src="javascript/jquery.apanel.js"></script>
              <script src="javascript/jquery.filebrowser.js"></script>      
              <script src="javascript/jquery.colorbox.js"></script> 
              <script src="javascript/jquery.colorpicker.js"></script>  
              <script src="javascript/calendar/calendar.js"></script>  
              <script src="javascript/calendar/lang/calendar-'.$conf['langcode'].'.js"></script>  
              <script>
              var loading = "'.$lang['wait_up'].'";
              var saves   =  "'.$lang['all_save'].'";   
              var mout   =  "'.$sess['mout'].'"; 
              var salert = $.cookie("alerts");
              if (salert != "off") { 
                  $(function(){';
        echo '        '.$this->globalnotice();
        echo '    });
              }
              $(function(){
                  $.template = "'.$sess['skin'].'"; 
                  $.errors   = "'.$lang['all_error'].'";
                  $.system();
                  $("img, a").tooltip();  
                  $("#reload").reload(); 
                  $("#menu-toggle").click(function() {
                      $("#aside").toggle();
                      if($("#aside").is(":hidden")){   
                          $.cookie("menup","closed"); 
                          $("#menup").attr({src:"template/'.$sess['skin'].'/images/'.(($sess['mout']=='right') ? 'closed' : 'open').'.gif"});  
                      } else {  
                          $.cookie("menup","open");  
                          $("#menup").attr({src:"template/'.$sess['skin'].'/images/'.(($sess['mout']=='right') ? 'open' : 'closed').'.gif"});
                      }
                  });               
              }); 	
              </script>  
              </head>
              <body>  
              <div id="wrapper">
                  <div id="header">
                      <div id="hgroup">
                          <h1 id="logo">'.$lang['control_panel'].'</h1> 
                      </div> 
                  </div>
                  <ul id="nav">
                      <li id="nav-left">  
                          <a href="'.$conf['site_url'].'/" target="_blank">'.$lang['goto_site'].'</a>
                          <a href="index.php?ops='.$sess['hash'].'">'.$lang['goto_index'].'</a>';
        if ($wysiwyg == 'yes') {
            echo '            <a class="spaw-on" href="index.php?dn=nowys&amp;ops='.$sess['hash'].'">'.$lang['spaw_close'].'</a>'; 
        } else {           
            echo '            <a class="spaw-of" href="index.php?dn=yeswys&amp;ops='.$sess['hash'].'">'.$lang['spaw_open'].'</a>';
        }
        if (in_array($ADMIN_ID,$CHECK_ADMIN['admid']) || is_array($ADMIN_PERM_ARRAY) && in_array('filebrowser',$ADMIN_PERM_ARRAY)) {
            echo '            <a href="javascript:$.filebrowser(\''.$sess['hash'].'\',\'/\',\'\');">'.$lang['adm_f_browser'].'</a>';
        }
        echo '        </li>
                      <li id="nav-right">
                          <a href="index.php?dn=logout&amp;ops='.$sess['hash'].'">'.$lang['goto_logout'].'</a>  
                      </li>
                  </ul>
                  <div id="content">  
                      <table id="content-table"> 
                          <tr>';    
        if ($sess['mout'] == 'left') { 
            $this->aside_menu();
        }
        echo '                <td id="section" class="bill"> 
                                  <div class="article">';
    }

    function sitedn($resursing) {
        return $resursing = (preg_match("/^[a-zA-Z0-9_]+$/D",$resursing)) ? substr($resursing,0,12) : '';
    }   
            
   /**
    * Подвал шаблона оформления 
    */
    function globalend() {
        global $plugin,$a,$conf,$mods,$sess,$wysiwyg,$lang,$panelimg,$style,$PLATFORM,$ADMIN_ID;
        $sess['skin'] = (is_dir(ADMINDIR.'/template/'.$sess['skin'])) ? $sess['skin'] : SKIN_DEF; 
        if (isset($_COOKIE['menup']) && $_COOKIE['menup'] == 'closed') {
            $panelimg = '<img id="menup" src="template/'.$sess['skin'].'/images/closed.gif" alt="'.$lang['openclose'].'" />';
            $style    = 'display: none;';
        } else {
            $panelimg = '<img id="menup" src="template/'.$sess['skin'].'/images/open.gif" alt="'.$lang['openclose'].'" />';
            $style    = '';
        } 
        if (isset($plugin['text']) || isset($plugin['copy'])) {
            echo '<div class="plugin">';
            echo (isset($plugin['text'])) ? 'Plugin: '.$plugin['text'].'<br />' : '';
            echo (isset($plugin['copy'])) ? 'Copyright: '.$plugin['copy'].'<br />' : '';
            echo (isset($plugin['www'])) ? 'WWW: '.$plugin['www'].'<br />' : '';
            echo (isset($plugin['help'])) ? 'Help Use: '.$plugin['help'] : '';
            echo '</div>';
        }
        $start = explode(' ',TimeStart);
        $start = $start[0] + $start[1];
        $end = microtime();
        $end = explode(' ',$end); $end = $end[0] + $end[1];
        
        echo '                    </div>  
                              </td>';
        $label = @unserialize($conf['apanelset']);
        if ($sess['mout'] == 'right') {  
            $this->aside_menu(); 
        }
        echo '            </tr>  
                      </table>
                  </div> 
                  <div id="bot"></div>
              </div>  
              <div id="footer">
                  <a class="up" href="#" onClick="scrollTo(0,0); return false;">&#9650;</a>
                  <div>Powered by <a href="http://www.danneo.com/" target="_blank">Danneo CMS</a> v.'.$conf['version'].' <i>©</i> 2004 - '.NEWYEAR.'</div>
              </div>
              </body>
              </html>';
        $gzip = (@extension_loaded('zlib') && isset($GLOBALS['HTTP_SERVER_VARS']['HTTP_ACCEPT_ENCODING'])) ? 1 : 0;
        if ($gzip) {
            $gzipenc = false;
            if (strpos($_SERVER['HTTP_ACCEPT_ENCODING'],'x-gzip') !== false) {
                $gzipenc = 'x-gzip';
            }
            if (strpos($_SERVER['HTTP_ACCEPT_ENCODING'],'gzip') !== false) {
            	$gzipenc = 'gzip';
            }
            if ($gzipenc) {
                $contents = @ob_get_contents();
                @ob_end_clean();
                @header('Content-Encoding: '.$gzipenc);
                echo @gzencode($contents,$conf['gziplevel']);
            }
        }
        exit();
    } 
         
   /**
    * Блочное меню 
    */
    function aside_menu() { 
        global $plugin,$a,$conf,$mods,$sess,$wysiwyg,$lang,$panelimg,$style,
               $PLATFORM,$ADMIN_ID,$CHECK_ADMIN,$ADMIN_PERM_ARRAY,$_REQUEST; 
        if ($sess['mout'] == 'right') { 
        echo '                <td class="panelinterface" id="menu-toggle">'.$panelimg.'</td>'; 
        }
        echo '                <td id="aside" class="menuinterface bill" style="'.$style.'">';
            
        if (preparse($PLATFORM,THIS_ARRAY) == 1 && in_array('platform',$ADMIN_PERM_ARRAY)) {
            if (isset($_COOKIE[PCOOKIE])) {
                list($pid) = @unserialize($_COOKIE[PCOOKIE]);
                if (preparse($pid,THIS_INT) > 0 && isset($PLATFORM[$pid])) {
        echo '                    <div class="menupanelin">'.$lang['all_plat'].'</div>
                                  <div class="menu-platform">'.$PLATFORM[$pid]['name'].'</div>';
                }
            }
        }
        
        echo '                    <div class="menupanelin">'.$lang['all_content'].'</div> 
                                  <div class="menu-content">';
        
        $m = $this->panelout();
        for ($i=0; $i < sizeof($m[0]); $i++) {
            if ($m[0][$i]) {
                $block = '';
                @include('system/navigation/'.$m[0][$i]);
                if (isset($block) && is_array($block) && isset($block['title']) && isset($block['id'])) {
                    $class = (isset($_COOKIE['openmenu']) && $_COOKIE['openmenu'] == $block['id']) ? ' menupanelopen' : '';
        echo '                    <div class="panels menupanel'.$class.'" id="'.$block['id'].'">';   
        if ($sess['icon'] == 'yes') {
        echo '                        <img src="template/'.$sess['skin'].'/images/menu/'.$block['id'].'.gif" alt="'.$block['title'].'" /> ';
        }
        echo '                        <span>'.$block['title'].'</span>
                                  </div>';
            
                    $display = (isset($_COOKIE['openmenu']) && $_COOKIE['openmenu'] == $block['id']) ? 'inline' : 'none';
        echo '                    <div class="mcont" style="display:'.$display.'">';
                    if (isset($block['link']) && is_array($block['link']))
                    {
                        while (list($link, $text) = each($block['link'])) 
                        {
                            if (is_array($text)) 
                            {
        echo '                        <a class="interface window-box" href="'.$link.'">'.$text[0].'</a>';
                            } else {  
                                      $act = ($_SERVER['REQUEST_URI'] == '/apanel/'.preg_replace("%&amp;%","&",$link)) ? ' id="mactive"' : '';
        echo '                        <a'.$act.' class="interface" href="'.$link.'">'.$text.'</a>';
                            }
                        }
                    } 
                                                   
        echo '                    </div>';
                }
                
            }
        }
        
        echo '                    </div>  
                                  <div class="menupanelin">'.$lang['all_system'].'</div>  
                                  <div class="menu-system">';
        
        for ($i=0; $i < sizeof($m[1]); $i++) {
            if ($m[1][$i]) {
                $block = '';
                @include('system/navigation/'.$m[1][$i]);
                if (isset($block) && is_array($block) && isset($block['title']) && isset($block['id']))
                {
                    $class = (isset($_COOKIE['openmenu']) && $_COOKIE['openmenu'] == $block['id']) ? ' menupanelopen' : '';
        echo '                    <div class="panels menupanel'.$class.'" id="'.$block['id'].'">'; 
        if ($sess['icon'] == 'yes') {
        echo '                        <img src="template/'.$sess['skin'].'/images/menu/'.$block['id'].'.gif" alt="'.$block['title'].'" /> ';
        }
        echo '                        <span>'.$block['title'].'</span>
                                  </div>';
            
                    $display = (isset($_COOKIE['openmenu']) && $_COOKIE['openmenu'] == $block['id']) ? 'inline' : 'none';
        echo '                    <div class="mcont" style="display:'.$display.'">';
                    if (isset($block['link']) && is_array($block['link']))
                    {
                        while (list($link, $text) = each($block['link'])) 
                        {
                            if (is_array($text)) 
                            {
        echo '                        <a class="interface window-box" href="'.$link.'">'.$text[0].'</a>';
                            } else {       
                                      $act = ($_SERVER['REQUEST_URI'] == '/apanel/'.preg_replace("%&amp;%","&",$link)) ? ' id="mactive"' : '';
        echo '                        <a'.$act.' class="interface" href="'.$link.'">'.$text.'</a>';
                            }
                        }
                    }
        echo '                    </div>';
                }
                
            }
        }
            
        echo '                    </div>  
                              </td>';  
        if ($sess['mout'] == 'left') { 
        echo '                <td class="panelinterface" id="menu-toggle">'.$panelimg.'</td>'; 
        }
    }  
         
   /**
    * Информационные сообщения, всплывающие 
    */
    function globalnotice() {              
        global $a,$conf,$sess,$wysiwyg,$lang,$PLATFORM,$ADMIN_ID,$CHECK_ADMIN,$ADMIN_PERM_ARRAY; 
        
	if ($ADMIN_ID == 1)  // показывать только для главного админа 
        {
	    $alert  = array();  
	    $i = 0;
	    // если не установлен Curl
            if (defined('NOTCURL')) {
        	$alert[$i]['title'] = $lang['isset_error'];
		$alert[$i]['desc']  = preparse_lga($lang['not_curl']);
		$alert[$i]['ico']   = 'template/'.$sess['skin'].'/images/iwarn.png';
		$alert[$i]['class'] = '';
		$i++; 
            } 
            // время оптимизировать базу
            if ((in_array('base',$ADMIN_PERM_ARRAY) || in_array($ADMIN_ID,$CHECK_ADMIN['admid'])) && ($conf['lastopt'] + 604800) < NEWTIME) {
        	$alert[$i]['title'] = $lang['all_attention'];
		$alert[$i]['desc']  = preparse_lga(str_replace('{sess}',$sess['hash'],$lang['mess_optimize']));
		$alert[$i]['ico']   = 'template/'.$sess['skin'].'/images/iinfo.png'; 
		$alert[$i]['class'] = 'alert-info';
		$i++; 
            }
            // доп. условие cookie админа
            if ((in_array('amanage',$ADMIN_PERM_ARRAY) || in_array($ADMIN_ID,$CHECK_ADMIN['admid'])) && SALT_ADMIN == "123456") {
		$alert[$i]['title'] = $lang['all_attention'];
		$alert[$i]['desc']  = preparse_lga($lang['error_salt']);
		$alert[$i]['ico']   = 'template/'.$sess['skin'].'/images/iwarn.png'; 
		$alert[$i]['class'] = 'alert-pass';
		$i++; 
            } 
            // секретное слово 
            if ((in_array('amanage',$ADMIN_PERM_ARRAY) || in_array($ADMIN_ID,$CHECK_ADMIN['admid'])) && $CHECK_ADMIN['sword'] == 'qwerty') {
		$alert[$i]['title'] = $lang['all_attention'];
		$alert[$i]['desc']  = preparse_lga($lang['mess_permiss']);
		$alert[$i]['ico']   = 'template/'.$sess['skin'].'/images/iwarn.png'; 
		$alert[$i]['class'] = 'alert-pass';
		$i++; 
            }
            
	    if (count($alert) == 0) {
		$alert = false;
	    }
	    
	    $result = '';    
	    if (is_array($alert)) {
		foreach ($alert as $k => $out) {
		    $result .= "            globalnotice('".$out['title']."', '".$out['desc']."', '".$out['ico']."', '".$out['class']."');\n"; 
		}
	    } 
	    print $result; 
	}
        return false;
    }

       
   /**
    * Панель блочное меню 
    */
    function panelout() {
        $readdir = opendir('system/navigation/');
        if (!is_resource($readdir)) echo 'I can not open dir <strong>/system/navigation/</strong> !!!';
        $listing = $apanel = array();
        while ($name = readdir($readdir)) {
            if (substr($name,0,5) == 'menu.' && substr($name,-4) == '.php') {
                $listing[] = $name;
            }
            
            if (substr($name,0,7) == 'apanel.' && substr($name,-4) == '.php') {
        	$apanel[] = $name;
            }
        }
        closedir($readdir);
        sort($listing);
        sort($apanel);
        return array($listing,$apanel);
    }

   /**
    * Панель верхнее меню, в разделах 
    */
    function globalmenu($title, $links, $addlinks = false) { 
         echo '<div class="nav">';  
        if ($addlinks) {
         echo '    <div class="addlink">'.$addlinks.'</div>';
        } 
         echo '    '.$links.'
               </div>'; 
    }

   /**
    * Страница аутентификации (вход в панель) 
    */
    function login($opsss = false) {    
        global $lang,$opsss,$LIFE_ADMIN;  
        echo '<!DOCTYPE html>
              <html>
              <head>
              <meta charset="'.CHAR_DEF.'"> 
              <title>'.$lang['control_panel'].'</title>
              <link rel="stylesheet" href="template/'.SKIN_DEF.'/css/login.css?v.'.VERSION.'">
              </head>
              <body>';   
        if (is_dir('../setup/')) {
        echo '<div><p>'.$lang['delete_setup'].' <span>setup</span></p></div>';   
        } else {
        echo '<div>
                  <h1>DANNEO <strong>CMS</strong></h1> 
                  <form action="index.php" method="post">
                  <label for="login">'.$lang['login'].'</label>
                  <input type="text" name="adlog" id="login" maxlength="15" autofocus="autofocus"><br>
                  <label for="password">'.$lang['password'].'</label>
                  <input type="password" name="adpwd" id="password" maxlength="15"><br>
                  <input class="blogin" type="submit" value="'.$lang['enter'].'">
                  </form>'; 
            if ($opsss == 1) { 
        echo '    <cite>'.$lang['auth_error'].'</cite>';
            } elseif ($opsss == 2) {
        echo '    <cite>'.str_replace('{lifeadmin}',$LIFE_ADMIN,$lang['sess_outdated']).'</cite>'; 
            } elseif ($opsss == 3) { 
        echo '    <cite>'.$lang['non_cookie'].'</cite>'; 
            } elseif ($opsss == 4) { 
        echo '    <cite>'.$lang['bad_agent'].'</cite>'; 
            } 
        echo '    <noscript><cite>'.$lang['noscript'].'</cite></noscript>';
            if ($opsss != 3) { 
        echo '    <script> 
                     if (!window.navigator.cookieEnabled) {
                          document.write("<cite>'.$lang['bad_cookie'].'</cite>");
                      }
                  </script>';
            }
        echo '</div>
              <strong>POWERED BY <a href="http://danneo.com" target="_blank">CMS DANNEO</a> '.VERSION.' <i>©</i> '.date('Y').'</strong>';  
             }
        echo '</body>
              </html>';
        exit();
    }

   /**
    * Всплывающие подсказки 
    */
    function outhint($hint) {
        global $sess;
        echo '<p class="hint"><img src="template/'.$sess['skin'].'/images/hicon.gif" alt="'.$hint.'" /></p>';
    }

   /**
    * Кнопка транслита для ЧПУ 
    */
    function outtranslit($gui, $obj, $hint) {
        global $sess;
        echo '&nbsp;<a class="but" href="javascript:$.translit(\''.$gui.'\',\''.$obj.'\')" title="'.$hint.'">&#8249;</a>';
    }

   /**
    * Обработка textarea 
    */
    function textarea($name, $rows, $cols, $value, $resize, $hint = false, $class = false, $req = false) {
        global $sess;
        $name = ($name) ? trim($name) : '';
        $value = ($value) ? notslashes(trim($value)) : '';
        $rowclass  = ($resize) ? ' class="textr resize {class}"' : ' class="textr noresize {class}"'; 
        $req = ($req) ? ' required="required"' : '';
        if ($class) {
            $rowclass = str_replace('{class}',$class,$rowclass);
        } else {
    	    $rowclass = str_replace('{class}','',$rowclass);
        }
        echo '<textarea name="'.$name.'" id="'.$name.'" rows="'.$rows.'" cols="'.$cols.'"'.$rowclass.''.$req.'>'.$value.'</textarea>';
        if ($hint) {
            echo ' '.$hint;
        }
    }
       
   /**
    * Предупреждения 
    */
    function globalalert($error, $message) {  
        echo '<div class="attention"> 
              <div class="attention-title">'.$error.'!</div>
                  <div class="attention-text">
                      '.$message.'
                  </div>
              </div>';
    }
    
   /**
    * Сообщение об удалении 
    */
    function shortdel($message) {
        global $lang,$sess;    
        echo '<div class="attention"> 
                  <div class="attention-title">'.$lang['all_alert'].'!</div>
                  <div class="attention-text">
                      '.$message.'
                  </div>
              </div>';
    }

   /**
    * Сообщение об ошибках 
    */
    function globalerror($message) {
        global $lang;   
        echo '<div class="attention"> 
                  <div class="attention-title">'.$lang['all_error'].'!</div>
                  <div class="attention-text">
                      '.$message.' 
                      <p><a class="but" href="javascript:history.go(-1)">'.$lang['all_goback'].'</a></p>  
                  </div>   
              </div>';
    }
    // файл-браузер и ланг-браузер, ошибка доступа
    function fberror($message) { 
        global $lang;   
        echo '<div class="fb-err  attention"> 
                  <div class="attention-title">'.$lang['all_error'].'!</div>
                  <div class="attention-text">
                      '.$message.'  
                  </div>   
              </div>';
    }
    //

   /**
    * Сообщение об ошибках в колорбокс 
    */
    function globalerrorbox($message){
        global $lang; 
        echo '<div class="box-err">
                  '.$message.'
              </div>';
    }

    function blankstart() {
        global $conf,$title,$lang,$sess; 
        echo '<!DOCTYPE html>
              <html>
              <head>
              <meta charset='.$conf['langcharset'].'">
              '.((isset($title)) ? '<title>'.$title.'</title>' : '<title>CMS Danneo '.$conf['version'].' / Apanel</title>').'
              </head>
              <body>';
    }

    function blankend() {
        echo '</body>
              </html>';
        exit();
    }

    function filter($act,$arr,$mod) {
        global $conf,$db,$basepref,$lang,$sess,$vals;  
        echo '<div id="filter" class="none">
              <form action="'.$act.'" method="post">   
              <div class="section">
              <table class="work">    
                  <caption>'.$mod.'&nbsp; &#8260; &nbsp;'.$lang['search'].'</caption>
                  <tr>
                      <th class="ar">'.$lang['all_filter'].'</th> 
                      <th>'.$lang['all_value'].'</th>
                  </tr>';
        foreach ($arr as $k => $v) {
        echo '    <tr>
                      <td>'.(isset($lang[$v[1]]) ? $lang[$v[1]] : $v[1]).'</td>
                      <td>';
            if ($v[2] == 'input') {
        echo '            <input type="text" name="filter['.$k.']" size="70" value="">';
            }
            if ($v[2] == 'checkbox') {
        echo '            <input type="checkbox" name="filter['.$k.']" value="1">';
            }  
            if ($v[2] == 'type') {   
        echo '            <select class="sw250" name="filter['.$k.']">'; 
                 foreach ($v[0] as $sk => $sv) {       
        echo '                <option value="'.$v[3][$sk].'">'.$lang[$sv].'</option>';
                 } 
        echo '            </select>';
            } 
            if ($v[2] == 'date') {
        echo '            <input type="text" id="filter0-'.$v[0].'" name="filter['.$k.'][0]" size="18" value="">';
                Calendar('filter0_'.$v[0],'filter0-'.$v[0]);
        echo '            <input type="text" id="filter1-'.$v[0].'" name="filter['.$k.'][1]" size="18" value="">';
                Calendar('filter1_'.$v[0],'filter1-'.$v[0]);
            } 
            if ($v[2] == 'access') {
        echo '            <select class="sw250" name="filter['.$k.']"> 
                              <option value="0">&#8212;</option>
                              <option value="all">'.$lang['all_all'].'</option>
                              <option value="user">'.$lang['all_user_only'].'</option>';  
        echo '            </select>';
            } 
        echo '        </td>
                  </tr>';
        }
        echo '    <tr class="tfoot">
                      <td></td>
                      <td class="al">
                          <input accesskey="s" class="but" value="'.$lang['all_apply'].'" type="submit">
                      </td>
                  </tr>
              </table>
              </div>
              </form>
              <div class="pad"></div>  
              </div>';
    }

}
$tm = new tm();
?>
