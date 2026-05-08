jQuery.rusid = new Array('А','Б','В','Г','Д','Е','Ё','Ж','З','И','Й',
                         'К','Л','М','Н','О','П','Р','С','Т','У','Ф',
                         'Х','Ч','Ц','Ш','Щ','Э','Ю','Я','Ы','Ъ','Ь',
                         'а','б','в','г','д','е','ё','ж','з','и','й',
                         'к','л','м','н','о','п','р','с','т','у','ф',
                         'х','ч','ц','ш','щ','э','ю','я','ы','ъ','ь',
                         ' ','\'','"','(',')','[',']',':','/','.','-',
                         '№','!','@','#','$','%','?','<','>','«','»',
                         ',','¤','¦','§','©','¬','®','°','±','µ','¶',
                         '·','&','‘','’','‚','“','”','„','†','‡','‰',
                         '‹','›','€','•','…','™','^','—','{','}','\\',
                         ';','_','+','=','~','`','|','*');
jQuery.latid = new Array('A','B','V','G','D','E','Jo','Zh','Z','I','J',
                         'K','L','M','N','O','P','R','S','T','U','F','H',
                         'Ch','C','Sh','Csh','E','Ju','Ja','Y','','',
                         'a','b','v','g','d','e','jo','zh','z','i','j',
                         'k', 'l','m','n','o','p','r','s','t','u','f','h',
                         'ch','c','sh','csh','e','ju','ja','y','','',
                         '-','','','','','','','','-','-','-',
                         '-','-','-','-','-','-','','-','-','','',
                         '-','-','-','-','-','-','-','-','-','-','-',
                         '-','-','-','-','-','','','','-','-','-',
                         '-','-','-','-','-','-','-','-','-','-','-',
                         '-','-','-','-','-','-','-','-');

jQuery.cookie = function(name, value, options){
    if (typeof value != 'undefined') { // name and value given, set cookie
        options = options || {};
        if (value === null) {
            value = '';
            options.expires = -1;
        }
        var expires = '';
        if (options.expires && (typeof options.expires == 'number' || options.expires.toUTCString)) {
            var date;
            if (typeof options.expires == 'number') {
                date = new Date();
                date.setTime(date.getTime() + (options.expires * 24 * 60 * 60 * 1000));
            } else {
                date = options.expires;
            }
            expires = '; expires=' + date.toUTCString();
        }

        var path = options.path ? '; path=' + (options.path) : '';
        var domain = options.domain ? '; domain=' + (options.domain) : '';
        var secure = options.secure ? '; secure' : '';
        document.cookie = [name, '=', encodeURIComponent(value), expires, path, domain, secure].join('');
    } else {
        var cookieValue = null;
        if (document.cookie && document.cookie != '') {
            var cookies = document.cookie.split(';');
            for (var i = 0; i < cookies.length; i++) {
                var cookie = jQuery.trim(cookies[i]);
                if (cookie.substring(0, name.length + 1) == (name + '=')) {
                    cookieValue = decodeURIComponent(cookie.substring(name.length + 1));
                    break;
                }
            }
        }
        return cookieValue;
    }
};

jQuery.pagescroll = function () {
    var xScroll, yScroll;
    if (self.pageYOffset) {
        yScroll = self.pageYOffset;
        xScroll = self.pageXOffset;
    } 
    else if (document.documentElement && document.documentElement.scrollTop) {
        yScroll = document.documentElement.scrollTop;
        xScroll = document.documentElement.scrollLeft;
    } 
    else if (document.body) {
        yScroll = document.body.scrollTop;
        xScroll = document.body.scrollLeft;
    }
    var arrayPageScroll = {'xScroll':xScroll,'yScroll':yScroll};
    return arrayPageScroll;
}

jQuery.reposit = function (layer) {
    var pageScroll = jQuery.pagescroll();
    var nHeight = parseInt($(layer).height(),10);
    var nWidth = parseInt($(layer).width(),10);
    var nTop = pageScroll.yScroll + ($(window).height() - nHeight) / 2;
    var nLeft = pageScroll.xScroll + ($(window).width() - nWidth) / 2;
    $(layer).css({left:nLeft,top:nTop});
    //$(layer).animate( {left:nLeft,top:nTop},{duration:100 } );
    $("#bg-overlay").css({width:$(document).width(),height:$(document).height()});
}

jQuery.createoverlay = function() {
    $('body').append('<div id="bg-overlay"></div><div id="bg-overlay-content"><img src="template/' + jQuery.template + '/images/loading.gif"></div>');
    $("#bg-overlay,#bg-overlay-content").hide();
    var $body = $(navigator.userAgent.indexOf('MSIE 6') >= 0 ? document.body : document);
    $('#bg-overlay').css({width:$body.width(),height:$body.height(),position:'absolute',top:'0px',left:'0px','opacity':0.6});
    jQuery.reposit('#bg-overlay-content');
    $(window).scroll(function(){ 
        jQuery.reposit('#bg-overlay-content'); 
    });
    $("#bg-overlay").click( function() {
        $("#bg-overlay,#bg-overlay-content").hide();
    });
}

jQuery.system = function(){
    $(".button").hover( function() {
        $(this).toggleClass("button").toggleClass("active-button");
    },
    function() {
        $(this).toggleClass("active-button").toggleClass("button");
    });
    $("input[type=text], textarea").focus( function() {
        $(this).toggleClass("").toggleClass("active-input");
    });
    $("input[type=text], textarea").blur( function() {
        $(this).toggleClass("active-input").toggleClass("");
    });
    $("#checkboxall").click( function() { 
        var checked_status = this.checked;
        $("input[type=checkbox]").each( function() { 
            this.checked = checked_status; 
        });
    });
};

jQuery.modcheck = function(id){
    var status = $('#' + id + 'checkbox').is(':checked');
    $("#" + id +"_toggle input[type=checkbox]").each( function() { 
        this.checked = status; 
    });
}

jQuery.ajaxget = function(url) {
    $("#ajaxbox").hide();
    $.colorbox({
  	onLoad: function() {
            $('#cboxClose').hide();
        },
        opacity:       '0',  
        initialWidth:  '40px',
        initialHeight: '40px',
        width:         '63px',
        height:        '69px',
        html:          '&nbsp;'
    });
    $("#cboxLoadingOverlay").css({"background":"#F5F9FC",opacity:0.9});
    $.ajax({
        url  : url,
        data : {},
        success: function(data) {
   	      $("#ajaxbox").html(data).show();
   	      $.fn.colorbox.close();
   	      $('#cboxClose').hide();
   	      $("#checkboxall").click(function(){
   	         var checked_status = this.checked;
                 $("input[type=checkbox]").each( function() {
                     this.checked = checked_status;
                 });
            }); 
        }  
    });
 
}

jQuery.ajaxeditor = function(url,id,w) {   
    if($("#ajaxpanel").length == 0){   
        $("body").append("<div id='ajaxpanel' class='ajaxpanel'></div>");  
    }          
    $("#ajaxpanel").hide();       
    var obj = $("#" + id), p = obj.position();       
    $("#ajaxpanel").css("left",(p.left) + 241 + "px")
                   .css("top",(p.top + 128) + "px") 
                   .animate({  
                           width: w + "px",  
                           fontSize: "1.2em",  
                           height: "23px",  
                           opacity: 1,    
                   });                  
    $.get(url, function(data){
        $("#ajaxpanel").html(data);   
    });        
    $("#section").click(function() { 
        $("#ajaxpanel").animate({fontSize:0,width:0,height:0,opacity:0});  
        $("#ajaxpanel").hide();  
        $("#ajaxpanel").clearQueue();    
    });      
}
jQuery.posteditor = function(form,id,url){
    var str = $(form).serialize();  
    $("#ajaxpanel").html('<span class="loads"></span>');
    $.post(url, str, function(data){    
        $("#ajaxpanel").animate({fontSize:0,width:0,height:0,opacity:0});   
        $("#ajaxpanel").hide(); 
        $("#ajaxpanel").clearQueue();  
        $("#" + id).html(data);
    });
    return false;
}

jQuery.translit = function(gui,obj) {
    var str = $("#" + gui).attr('value'); 
    // переводим в нижний регистр, удаляем двойные пробелы, пробелы с запятой и пробелы по краям 
    var str = str.toLowerCase().replace(/\s+/g,' ').replace(/(^\s*)|(\s*)$/g, '').replace(/(?:,)\s/g, ' '); 
    if (str){
        var chars;
        var re = '';
        for (i=0; i < str.length; i++) {
            chars = str.charAt(i,1);
            var me = false;
            for (a=0; a<this.rusid.length; a++) {
                if (chars == this.rusid[a]) {
                    me = true;
                    break;
                }
            }
            re += (me) ? this.latid[a] : chars;
        }
        $("#" + obj).attr({value:re});
    }
}

jQuery.windows = function(url,name,width,height,scroll) {
    var tl = '';
    if (width < 0) { 
        width = $(window).width() + width; 
    }
    if (height < 0) { 
        height = $(window).height() + height; 
    }
    if (width) { 
        tl+= ',left=' + ($(window).width() - width) / 2; 
    }
    if (height) { 
        tl+= ',top=' + ($(window).height() - height) / 2; 
    }
    window.open(url,name,'width=' + ((width) ? width : 'auto') + ',height=' + ((height) ? height : 'auto') + ',dependent=yes,titlebar=no,status=no,scrollbars=' + ((scroll) ? scroll : 'no') + tl);
}

jQuery.openurl = function(url) {
    window.location = url;
}

jQuery.addtaginput = function(form,area) {
    var id = $("#countid").attr('value');
    if (id) {
        id++;
        var html = '<div class="section tag" id="taginput' + id + '" style="display:none;">';
        html+= '<table class="work"><tr>';
        html+= '<td>' + all_name + ', ' + all_cpu + '</td>';
        html+= '<td>';
        html+= '<input type="text" name="tagword[' + id + ']" id="tagword' + id + '" size="15" maxlength="255">&nbsp;';
        html+= '<a class="but" href="javascript:$.translittag(\'tagword' + id + '\',\'tagcpu' + id + '\',\''+ form + '\',\'' + area + '\');">URL</a>&nbsp;';
        html+= '<input type="text" name="tagcpu[' + id + ']" id="tagcpu' + id + '" size="15" maxlength="255">&nbsp;';
        html+= '<a class="but" href="javascript:$.removetaginput(\'' + form + '\',\'' + area + '\',\'taginput' + id + '\');">x</a>';  
        html+= '</td></tr><tr>';
        html+= '<td>' + all_popul + '</td>';
        html+= '<td>';
        html+= '<input type="text" name="tagrating[' + id + ']" id="tagrating[' + id + ']" size="3" maxlength="3" value="0">';
        html+= '</td>';
        html+= '</tr>'; 
        html+= '</table>';
        html+= '</div>';
        if (typeof page != 'undefined') {
            html+= '<script type="text/javascript">';
            html+= '$(function() {';
            html+= '$("#tagword' + id + '").autocomplete({url:"' + page + '.php?dn=autocomplete&ops=' + ops + '",onItemSelect:function(item){ $("#tagcpu' + id + '").attr("value", item.data);}});';
            html+= '});';
            html+= '</script>';
        }
        $("form[name="+ form +"] #" + area).append(html);
        $("form[name="+ form +"] #" + area + " #taginput" + id).show('normal');
        $("#countid").attr({value:id});
    }
}

jQuery.removetaginput = function(form,area,id){
    $("form[name="+ form +"] #" + area + " #" + id).hide('normal', function() {
        $("form[name="+ form +"] #" + area + " #" + id).remove();
    });
}

jQuery.translittag = function(gui,obj,form,area) {
    var str = $("form[name="+ form +"] #" + area + " #" + gui).attr('value');
    if (str) {
        var chars;
        var re = '';  
        for (i = 0; i < str.length; i++) {
            chars = str.charAt(i,1);
            var me = false;
            for (a = 0; a < this.rusid.length; a++) {
                if (chars == this.rusid[a]) {
                    me = true;
                    break;
                }
            }
            re += (me) ? this.latid[a] : chars;
        }
        $("form[name="+ form +"] #" + area + " #" + obj).attr({value:re});
    }
}

jQuery.changeselect = function(sel) {
    if ($(sel).length > 0) {
        if (ajax == 1) {
            jQuery.ajaxget($(sel).attr('value'));
        } else {
            jQuery.openurl($(sel).attr('value'));
        }
    }
}

jQuery.insertinfo = function(obj,tag) {
    var newobj = document.getElementById(obj), tag = '{' + tag + '}';
    if (newobj) {
        if (document.selection) {
            newobj.focus();
            document.selection.createRange().duplicate().text = tag;
        } 
        else if (newobj.selectionStart || newobj.selectionStart == '0') {
            var selEnd = newobj.selectionEnd, txtLen = newobj.value.length;
            var txtbefore = newobj.value.substring(0,selEnd), txtafter =  newobj.value.substring(selEnd,txtLen);
            newobj.value = txtbefore +  tag + txtafter;
        } else {
            newobj.text.value += tag;
        }
    }
}

jQuery.langbrowser = function(sess){
    $.ajax({
        //async:false,
        cache: false,
        url:   'langbrowser.php',
        data:  'ops=' + sess + '&dn=index',
        error: function(msg) {
        },
            success: function(data) {
                if (data.length > 0) {
                    $.colorbox({ 
                                 width     : '92%',
                                 height    : '657px',   
                                 maxHeight :  657,
                                 maxWidth  :  1200,
                                 html      :  data,
                                 onComplete: function () {
                                     var $h = $('#cboxLoadedContent').height();
                                     $('#lang-scroll').css({'height' : ($h - 10) + 'px'});
                                 }
                    });
                    $('#lang-scroll').html(data);
                }
            }
    });
}

jQuery.langbrowserupdate = function(sess,id){
    $.ajax({
        //async:false,
        cache: false,
        url:   'langbrowser.php',
        data:  'ops=' + sess + '&langsetid=' + id,
        error: function(msg) {
        },
            success: function(data) {
                if (data.length > 0) {
                    $('#lang-scroll').html(data).show();
            }
        }
    });
} 
